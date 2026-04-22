extern crate core;

use crate::camera::{Camera, CameraController};
use crate::kml_viewer_group::KmlGroup;
use crate::puck_group::SimplePuck;
use crate::route::RouteCosting;
use crate::tiles::tile_data::TileData;
use crate::tiles::tiles_provider::{TilesMessage, TilesProvider};
use futures::executor::block_on;
use futures::{pin_mut, Stream, StreamExt};
use geo_types::private_utils::get_bounding_rect;
use geo_types::{coord, Coord, Point, Rect};
use geo_types::{LineString, Polygon};
use glam::{DVec2, DVec3, Vec2};
use num::{abs, clamp};
use osm::styles::style_loader::StyleLoader;
use osm::styles::{DashStyle, RenderStyle};
use renderer::canvas_api::CanvasApi;
use renderer::mesh_layers::feature_layers::FeatureLayerTag;
use renderer::modifier::render_modifier::SpatialData;
use renderer::render_group::RenderGroup;
use renderer::renderer_api::RendererApi;
use renderer::styles::style_id::StyleId;
use renderer::{Renderer, RendererUpdateData, ShashlikRenderer};
use route::route_controller::RouteController;
#[cfg(feature = "sgnss")]
use sgnss::{start_sgnss};
use std::mem;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, LazyLock};
use std::thread::spawn;
use std::time::{Duration, Instant};
use ttf_parser::Face;
use wgpu::Texture;
use wgpu_canvas::wgpu_canvas::WgpuCanvas;

mod camera;
pub mod feature_processor;
mod kml_viewer_group;
pub mod mesh_loader;
mod puck_group;
pub mod route;
pub mod tiles;
pub struct ShashlikMap<T: TilesProvider> {
    renderer: Box<ShashlikRenderer>,
    camera: Camera,
    camera_controller: CameraController,
    tiles_provider: T,
    route_controller: RouteController,
    last_area_lon_lat: Rect,
    current_world_position: DVec3,
    current_bearing: f64,
    current_pitch: f64,
    cam_follow_mode: bool,
    cam_follow_zoom_lock: Option<f64>,
    screen_params: ScreenParam,
    map_event_receiver: Receiver<MapEvent>,
    last_interaction: Instant,
}

enum MapEvent {
    LatLon(f64, f64),
}

struct ScreenParam {
    width: u32,
    height: u32,
}

impl ScreenParam {
    fn ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    fn center(&self) -> Vec2 {
        Vec2::new(self.width as f32, self.height as f32) * 0.5f32
    }
}

impl RenderGroup for TileData {
    fn content(&mut self, canvas: &mut CanvasApi) {
        mem::take(&mut self.geometry_data)
            .into_iter()
            .for_each(|data| {
                canvas.geometry_data(data);
            });
    }
}
static DEFAULT_FONT: LazyLock<Face, fn() -> Face<'static>> =
    LazyLock::new(|| Face::parse(include_bytes!("../font.ttf"), 0).unwrap());

impl<T: TilesProvider> ShashlikMap<T> {
    const TEMP_ANIMATION_SPEED: f64 = 0.03;

    const FOLLOW_ANIMATION_DELAY_MS: u64 = 2000;

    pub async fn new(
        canvas: Box<dyn WgpuCanvas>,
        mut tiles_provider: T,
    ) -> anyhow::Result<ShashlikMap<T>> {
        let screen_size = (canvas.config().width as f32, canvas.config().height as f32);

        let feature_layer_tags = vec![
            FeatureLayerTag {
                name: "kml_layer",
                ..Default::default()
            },
            FeatureLayerTag {
                name: "route_layer",
                vertex_shader: Some("vs_main_route"),
                indirect: true,
            },
            FeatureLayerTag {
                name: "puck_layer",
                ..Default::default()
            },
        ];
        let renderer = ShashlikRenderer::new(feature_layer_tags, canvas, &DEFAULT_FONT).await?;
        let tiles_stream = tiles_provider.tiles();

        let initial_coord: Coord<f64> = (139.757080078125, 35.68798828125).into();
        let camera_offset = T::lon_lat_to_world(&initial_coord);
        let camera_offset: DVec3 = (camera_offset.x, camera_offset.y, 0.0).into();
        let cam = Camera::new(camera_offset);

        let mut puck_spatial_data = SpatialData::transform(DVec3::new(0.0, 0.0, 0.0));
        puck_spatial_data.scale(1.0);
        renderer.api.add_render_group(
            "puck".to_string(),
            puck_spatial_data,
            Box::new(SimplePuck {}),
        );

        Self::run_tiles(renderer.api.clone(), tiles_stream);

        let mut camera_controller = CameraController::new();
        camera_controller.pitch = 45.0;
        camera_controller.position = camera_offset;

        let (map_event_sender, map_event_receiver) = mpsc::channel();

        let mut map = ShashlikMap {
            renderer: Box::new(renderer),
            camera: cam,
            camera_controller,
            tiles_provider,
            route_controller: RouteController::new(),
            last_area_lon_lat: Rect::new((0.0, 0.0), (0.0, 0.0)),
            current_world_position: camera_offset,
            current_bearing: 0.0,
            current_pitch: 45.0,
            cam_follow_mode: true,
            cam_follow_zoom_lock: Some(30.0),
            screen_params: ScreenParam {
                width: screen_size.0 as u32,
                height: screen_size.1 as u32,
            },
            map_event_receiver,
            last_interaction: Instant::now(),
        };
        map.set_lon_lat_bearing(initial_coord.x, initial_coord.y, Some(0f32));
        map.load_styles();
        // FIXME, the first route rendering can take a lot of time...
        if cfg!(target_os = "linux") {
            map.create_route_to((initial_coord.x, initial_coord.y), RouteCosting::Auto);
        }


        Self::start_sgnss_if_available(map_event_sender);

        Ok(map)
    }

    pub fn clip_to_lon_lat(&self, coord: &Coord<f64>) -> Option<Coord<f64>> {
        let world_on_ground = self.renderer.clip_to_world(coord)?;
        Some(T::world_to_lon_lat(
            &(world_on_ground.x, world_on_ground.y).into(),
        ))
    }

    fn run_tiles(
        renderer_api: Arc<RendererApi>,
        tiles_stream: impl Stream<Item = TilesMessage> + Send + 'static,
    ) {
        spawn(move || {
            block_on(async {
                pin_mut!(tiles_stream);
                loop {
                    let item = tiles_stream.next().await;
                    match item {
                        None => break,
                        Some(msg) => match msg {
                            TilesMessage::TilesData(data) => {
                                data.into_iter().for_each(|item| {
                                    renderer_api.add_render_group(
                                        item.key.to_string(),
                                        SpatialData::transform(item.position).size(item.size),
                                        Box::new(item),
                                    );
                                });
                            }
                            TilesMessage::ToRemove(set) => {
                                renderer_api.clear_render_groups(set);
                            }
                        },
                    }
                }
            })
        });
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.camera.resize(width, height);
        self.renderer.resize(width, height);
        self.screen_params.width = width;
        self.screen_params.height = height;
    }

    pub fn update_and_render(&mut self) -> Option<Texture> {
        self.consume_map_events();
        self.camera_controller.update_camera(&mut self.camera);

        self.update_entities();

        let (view, view_proj) = self.camera.build_view_projection_matrix();
        let view_light = self.camera.build_view_light_matrix();

        let update_data = RendererUpdateData {
            view_matrix: view,
            view_light_matrix: view_light,
            proj_matrix: self.camera.perspective_matrix,
            view_proj_matrix: view_proj,
            cs_offset: self.camera.offset,
            scale: self.camera.scale(),
            eye_direction: self.camera.eye_direction(),
            up: self.camera.up
        };
        self.renderer.update(update_data);

        self.fetch_tiles();

        self.renderer.render()
    }

    fn fetch_tiles(&mut self) {
        let zoom_level = self.camera_controller.camera_z / 100.0;
        let zoom_level = (zoom_level.log2().round() as i32).max(0);
        let p1 = self.clip_to_lon_lat(&coord! {x: -1.0, y: -1.0}).unwrap();
        let p2 = self.clip_to_lon_lat(&coord! {x: 1.0, y: -1.0}).unwrap();
        let p3 = self.clip_to_lon_lat(&coord! {x: 1.0, y: 1.0}).unwrap();
        let p4 = self.clip_to_lon_lat(&coord! {x: -1.0, y: 1.0}).unwrap();

        // this will be compared for intersection later, it should have a correct winding
        let poly: Polygon<f64> = Polygon::new(LineString(vec![p1, p2, p3, p4]), Vec::new());
        let area_lon_lat = get_bounding_rect(poly.exterior()).unwrap();

        // if area_lon_lat != self.last_area_lon_lat {
        self.tiles_provider.load(area_lon_lat, poly, zoom_level);
        // }

        self.last_area_lon_lat = area_lon_lat;
    }

    fn consume_map_events(&mut self) {
        if let Ok(event) = self.map_event_receiver.try_recv() {
            match event {
                MapEvent::LatLon(lat, lon) => {
                    self.set_lon_lat_bearing(lon, lat, None);
                }
            }
        }
    }

    fn update_entities(&mut self) {
        let puck_location = self.current_world_position;
        let bearing = self.current_bearing;

        let cam_zoom = self.camera_controller.forward_len / 100.0;

        let cam_yaw = self.camera_controller.yaw;

        self.renderer
            .api
            .update_spatial_data("puck".to_string(), move |spatial_data| {
                spatial_data.scale = cam_zoom;
                spatial_data.transform +=
                    (puck_location - spatial_data.transform) * Self::TEMP_ANIMATION_SPEED;
                spatial_data.yaw +=
                    ((bearing - spatial_data.yaw) % 360.0) * Self::TEMP_ANIMATION_SPEED;
            });

        if self.should_animate() {
            let cam_pos = self.camera_controller.position;
            let cam_pos = DVec3::new(cam_pos.x, cam_pos.y, cam_pos.z);

            let transform_cam_offset = (self.current_world_position) - cam_pos;
            let transform_cam_offset_anim = transform_cam_offset * Self::TEMP_ANIMATION_SPEED;
            // TODO Animation framework. Now it just fixes teleport bug
            let new_cam_pos = if transform_cam_offset_anim.length() >= 300.0 {
                cam_pos + transform_cam_offset
            } else {
                cam_pos + transform_cam_offset_anim
            };

            self.camera_controller.set_new_position(new_cam_pos);

            let new_cam_yaw = cam_yaw + ((self.current_bearing - cam_yaw) % 360.0) * Self::TEMP_ANIMATION_SPEED;
            self.camera_controller.yaw = new_cam_yaw
        }

        if self.should_animate() {
            self.camera_controller.pitch +=
                (self.current_pitch - self.camera_controller.pitch) * Self::TEMP_ANIMATION_SPEED;

            if let Some(zoom_lock) = self.cam_follow_zoom_lock {
                let current_delta = self.camera_controller.forward_len - zoom_lock;
                if abs(current_delta) > 10.0 {
                    self.camera_controller.zoom_delta = current_delta * Self::TEMP_ANIMATION_SPEED;
                }
            }
        }
    }

    pub fn zoom_delta(&mut self, delta: f32, point: (f32, f32)) {
        self.reset_last_interaction();

        self.camera_controller.zoom_delta = delta as f64;

        let screen_center = self.screen_params.center();
        let diff = (Vec2::from(point) - screen_center) * 0.5f32;
        let px = diff.x / screen_center.x;
        let py = diff.y / screen_center.y;
        self.pan_delta(delta * px * self.screen_params.ratio(), delta * py);
    }

    pub fn pan_delta(&mut self, delta_x: f32, delta_y: f32) {
        self.reset_last_interaction();
        self.camera_controller.pan_delta = DVec2::new(delta_x as f64, delta_y as f64);
    }

    pub fn pitch_delta(&mut self, delta: f32) {
        self.reset_last_interaction();
        self.camera_controller.pitch = clamp(self.camera_controller.pitch + delta as f64, 45.0, 90.0);
    }

    fn reset_last_interaction(&mut self) {
        self.last_interaction = Instant::now();
    }

    fn should_animate(&self) -> bool {
        self.cam_follow_mode && Instant::now().duration_since(self.last_interaction)
            >= Duration::from_millis(Self::FOLLOW_ANIMATION_DELAY_MS)
    }

    pub fn set_camera_follow_mode(&mut self, follow_mode: bool) {
        self.cam_follow_mode = follow_mode;

        if self.cam_follow_mode {
            self.cam_follow_zoom_lock = Some(30.0);
            self.current_pitch = 45.0;
        }
    }

    pub fn set_current_pitch(&mut self, current_pitch: f64) {
        self.current_pitch = current_pitch;
    }

    pub fn set_cam_follow_zoom_lock(&mut self, cam_follow_zoom_lock: Option<f64>) {
        self.cam_follow_zoom_lock = cam_follow_zoom_lock;
    }

    pub fn set_lon_lat_bearing(&mut self, lon: f64, lat: f64, bearing: Option<f32>) {
        self.route_controller.set_current_lon_lat((lon, lat));
        let position = T::lon_lat_to_world(&coord! {x: lon, y: lat});
        self.current_world_position = DVec3::new(position.x, position.y, 0.0);
        if let Some(bearing) = bearing {
            let bearing = bearing as f64;
            let mut rot_diff = (bearing % 360.0) - (self.current_bearing % 360.0);
            if rot_diff.abs() > 180.0 {
                rot_diff -= rot_diff.signum() * 360.0;
            }
            self.current_bearing += rot_diff % 360.0;
        }
    }

    pub fn create_route_to_from_screen_center(&self, route_costing: RouteCosting) {
        let center = self.clip_to_lon_lat(&coord! {x: 0.0, y: 0.0}).unwrap();
        self.create_route_to(center.into(), route_costing);
    }

    pub fn create_route_to_screen_point(
        &self,
        point_x: f32,
        point_y: f32,
        route_costing: RouteCosting,
    ) {
        let clip = coord! {x: (point_x / self.screen_params.width as f32) as f64,
        y: (point_y / self.screen_params.height as f32) as f64};
        let clip = coord! { x: 2.0*(clip.x - 0.5), y: 2.0*(clip.y - 0.5) };
        let center = self.clip_to_lon_lat(&clip).unwrap();
        self.create_route_to(center.into(), route_costing);
    }

    pub fn create_route_to(&self, to_lon_lat: (f64, f64), route_costing: RouteCosting) {
        self.route_controller.calc_route(
            to_lon_lat,
            route_costing,
            self.create_location_coord_converter(),
            self.renderer.api.clone(),
        );
    }

    fn create_location_coord_converter(&self) -> Box<dyn (Fn(&Point) -> Point) + Send> {
        Box::new(move |p| {
            let coord: Coord<f64> = (p.x(), p.y()).into();
            let coord = T::lon_lat_to_world(&coord);
            Point::new(coord.x, coord.y)
        })
    }

    fn load_styles(&self) {
        StyleLoader::load().into_iter().for_each(|style| {
            let style_id = StyleId(Box::leak(style.id.into_boxed_str()));
            let actual_render_style = match style.render_style {
                RenderStyle::Fill(color) => {
                    renderer::styles::render_style::RenderStyle::fill(color.as_array())
                }
                RenderStyle::Border(color, percent) => {
                    renderer::styles::render_style::RenderStyle::border(color.as_array(), percent)
                }
                RenderStyle::Dashed(color1, color2, dash_style) => {
                    let dash_style_value = match dash_style {
                        DashStyle::Solid => 0,
                        DashStyle::Circles => 1,
                    };
                    renderer::styles::render_style::RenderStyle::dashed(
                        color1.as_array(),
                        color2.as_array(),
                        dash_style_value,
                    )
                }
            };
            self.renderer
                .api
                .update_style(style_id, move |style| *style = actual_render_style);
        });
    }

    pub fn load_kml_path(&self, path_buf: PathBuf) {
        println!("Loading KML from {:?}", path_buf);
        let kml_group = KmlGroup::new(path_buf, self.create_location_coord_converter());

        self.renderer.api.add_render_group(
            "kml_data".to_string(),
            SpatialData::transform(DVec3::new(0.0, 0.0, 0.0)),
            Box::new(kml_group),
        );
    }

    pub fn clear_routes(&self) {
        self.route_controller
            .clear_routes(self.renderer.api.clone());
    }

    #[allow(unused_variables)]
    fn start_sgnss_if_available(map_sender: Sender<MapEvent>) {
        #[cfg(feature = "sgnss")]
        start_sgnss(move |lat, lon| {
            map_sender.send(MapEvent::LatLon(lat, lon)).unwrap();
        });
    }
}
