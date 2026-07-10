extern crate core;

use crate::camera::{Camera, CameraController};
use crate::kml_viewer_group::KmlGroup;
use crate::puck_group::SimplePuck;
use crate::route::RouteCosting;
use crate::tiles::tile_data::TileData;
use crate::tiles::tiles_provider::{TilesProvider};
use crate::tiles::tiles_provider::TilesMessage;
use futures::executor::block_on;
use futures::{pin_mut, Stream, StreamExt};
use geo_types::{coord, Coord, Point};
use geo_types::{Polygon};
use glam::{DMat2, DVec2, DVec3, Vec2};
use num::{abs, clamp};
use osm::styles::style_loader::StyleLoader;
use osm::styles::{DashStyle, RenderStyle};
use renderer_common::render_modifier::SpatialData;
use renderer_common::render_group::RenderGroup;
use renderer_common::style_id::StyleId;
use route::route_controller::RouteController;
#[cfg(feature = "sgnss")]
use sgnss::start_sgnss;
use std::mem;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, LazyLock};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};
use log::error;
use ttf_parser::Face;
use renderer_common::{CanvasApi, RendererApi, Renderer, RendererUpdateData, SSAO_ENABLED};
use crate::transition_2d_3d_helper::Transition2d3dHelper;

mod camera;
pub mod feature_processor;
mod kml_viewer_group;
pub mod mesh_loader;
mod puck_group;
pub mod route;
pub mod tiles;
mod transition_2d_3d_helper;

pub struct ShashlikMap<R: Renderer, T: TilesProvider> {
    renderer: R,
    camera: Camera,
    camera_controller: CameraController,
    tiles_provider: T,
    route_controller: RouteController<R::RAPI>,
    current_world_position: DVec3,
    current_bearing: f64,
    camera_bearing: f64,
    current_pitch: f64,
    transition_2d_3d_helper: Transition2d3dHelper,
    cam_follow_mode: bool,
    cam_follow_zoom_lock: Option<f64>,
    screen_params: ScreenParam,
    map_event_receiver: Receiver<MapEvent>,
    last_interaction: Instant,
    world_width_on_screen: f64,
    world_height_on_screen: f64,
}

enum MapEvent {
    LatLon(f64, f64),
}

struct ScreenParam {
    width: u32,
    height: u32,
}

impl ScreenParam {
    fn center(&self) -> Vec2 {
        Vec2::new(self.width as f32, self.height as f32) * 0.5f32
    }
}

impl <T: CanvasApi> RenderGroup<T> for TileData {
    fn content(&mut self, canvas: &mut T) {
        mem::take(&mut self.geometry_data)
            .into_iter()
            .for_each(|data| {
                canvas.geometry_data(data);
            });
    }
}
pub static DEFAULT_FONT: LazyLock<Face, fn() -> Face<'static>> =
    LazyLock::new(|| Face::parse(include_bytes!("../font.ttf"), 0).unwrap());

// FIXME We should not hardcode it in general. But so far it's just a first step.
const MAX_ZOOM_LEVEL: i32 = 15;

impl<R: Renderer, T: TilesProvider + Sync> ShashlikMap<R, T> {
    const TEMP_ANIMATION_SPEED: f64 = 0.03;

    const FOLLOW_ANIMATION_DELAY_MS: u64 = 2000;
    const TELEPORT_THRESHOLD: f64 = 300.0;
    const ZOOM_LOCK_DIST: f64 = 200.0;

    pub async fn new(renderer: R, mut tiles_provider: T) -> anyhow::Result<ShashlikMap<R, T>> {
        let screen_size = renderer.screen_size();
        let tiles_stream = tiles_provider.tiles();

        let initial_coord: Coord<f64> = (139.757080078125, 35.68798828125).into();
        let camera_offset = tiles_provider.lon_lat_to_world(&initial_coord, MAX_ZOOM_LEVEL);
        let camera_offset: DVec3 = (camera_offset.x, camera_offset.y, 0.0).into();
        let cam = Camera::new(camera_offset.truncate());

        let mut puck_spatial_data = SpatialData::transform(DVec3::new(0.0, 0.0, 0.0));
        puck_spatial_data.scale(DVec3::splat(1.0));
        renderer.api().add_render_group(
            "puck".to_string(),
            puck_spatial_data,
            Box::new(SimplePuck {}),
        );

        let zero_zoom_level_loaded = Arc::new(AtomicBool::new(false));
        let transition_2d_3d_helper = Transition2d3dHelper::new(zero_zoom_level_loaded.clone());
        Self::run_tiles(renderer.api(), zero_zoom_level_loaded.clone(), tiles_stream);
        Self::load_styles(renderer.api());

        let mut camera_controller = CameraController::new();
        camera_controller.pitch = CameraController::MIN_PITCH;
        camera_controller.position = camera_offset;

        let (map_event_sender, map_event_receiver) = mpsc::channel();

        let route_controller = RouteController::new(renderer.api());
        let mut map = ShashlikMap {
            renderer,
            camera: cam,
            camera_controller,
            tiles_provider,
            route_controller,
            current_world_position: camera_offset,
            current_bearing: 0.0,
            camera_bearing: 0.0,
            current_pitch: CameraController::MIN_PITCH,
            transition_2d_3d_helper,
            cam_follow_mode: false,
            cam_follow_zoom_lock: None, //Some(Self::ZOOM_LOCK_DIST),
            screen_params: ScreenParam {
                width: screen_size.0 as u32,
                height: screen_size.1 as u32,
            },
            map_event_receiver,
            last_interaction: Instant::now(),
            world_width_on_screen: 0.0,
            world_height_on_screen: 0.0,
        };
        map.set_lon_lat_bearing(initial_coord.x, initial_coord.y, Some(0f32));


        Self::start_sgnss_if_available(map_event_sender);

        Ok(map)
    }

    fn clip_to_lon_lat(&self, coord: &Coord<f64>) -> Option<Coord<f64>> {
        let world_on_ground = self.renderer.clip_to_world(coord)?;
        Some(self.world_to_lon_lat(&world_on_ground))
    }

    fn world_to_lon_lat(&self, world_on_ground: &DVec2) -> Coord<f64> {
        self.tiles_provider.world_to_lon_lat(
            &(world_on_ground.x, world_on_ground.y).into(), MAX_ZOOM_LEVEL
        )
    }

    fn run_tiles(
        renderer_api: Arc<R::RAPI>,
        zero_zoom_level_loaded: Arc<AtomicBool>,
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
                                let has_zero_level = data.iter().any(|item|
                                    item.zoom_level == MAX_ZOOM_LEVEL
                                );
                                zero_zoom_level_loaded.store(has_zero_level, Ordering::Relaxed);
                                data.into_iter().for_each(|item| {
                                    renderer_api.add_render_group(
                                        item.key.to_string(),
                                        SpatialData::transform(item.position).bbox(item.bbox),
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

    pub fn update_and_render(&mut self) -> Option<R::OUTPUT> {
        self.consume_map_events();
        self.camera_controller.update_camera(&mut self.camera);

        self.update_entities();
        self.update_styles();

        let cam_zoom = self.camera.scale();
        let scale_2d_3d = self.transition_2d_3d_helper.update(cam_zoom, Self::TEMP_ANIMATION_SPEED as f32);

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
            up: self.camera.up,
            scale_2d_3d,
        };
        self.renderer.update(update_data);

        self.fetch_tiles();

        self.renderer.render()
    }

    fn fetch_tiles(&mut self) {
        let world_on_ground_center = self.renderer.clip_to_world(&coord! {x: 0.0, y: 0.0}).unwrap();
        let world_on_ground_left_top = self.renderer.clip_to_world(&coord! {x: -1.0, y: -1.0}).unwrap();
        let world_on_ground_left_bottom = self.renderer.clip_to_world(&coord! {x: -1.0, y: 1.0}).unwrap();
        let world_on_ground_right_bottom = self.renderer.clip_to_world(&coord! {x: 1.0, y: 1.0}).unwrap();
        let world_on_ground_right_top = self.renderer.clip_to_world(&coord! {x: 1.0, y: -1.0}).unwrap();

        let world_on_ground_center_left = self.renderer.clip_to_world(&coord! {x: -1.0, y: 0.0}).unwrap();
        let world_on_ground_center_right = self.renderer.clip_to_world(&coord! {x: 1.0, y: 0.0}).unwrap();

        let rotation_matrix = DMat2::from_angle(-self.camera_controller.yaw.to_radians());

        let world_on_ground_rotated_left_top = rotation_matrix * (world_on_ground_left_top - world_on_ground_center) + world_on_ground_center;
        let world_on_ground_rotated_bottom_right = rotation_matrix * (world_on_ground_right_bottom - world_on_ground_center) + world_on_ground_center;
        let world_on_ground_center_left = rotation_matrix * (world_on_ground_center_left - world_on_ground_center) + world_on_ground_center;
        let world_on_ground_center_right = rotation_matrix * (world_on_ground_center_right - world_on_ground_center) + world_on_ground_center;

        self.world_width_on_screen = (world_on_ground_center_left.x - world_on_ground_center_right.x).abs();
        self.world_height_on_screen = (world_on_ground_rotated_left_top.y - world_on_ground_rotated_bottom_right.y).abs();

        let zoom_level = self.camera.scale();
        let zoom_level = ((zoom_level.log2() - 1.0) as i32).max(0);
        let zoom_level = MAX_ZOOM_LEVEL - zoom_level;

        let poly_coords: Vec<Coord> = vec![world_on_ground_left_top,
                                           world_on_ground_right_top,
                                           world_on_ground_right_bottom,
                                           world_on_ground_left_bottom].into_iter().map(|coord| {
            coord! {x: coord.x, y: coord.y}
        }).collect();
        let poly = Polygon::new(poly_coords.into(), Vec::new());

        self.tiles_provider.load(poly, zoom_level);
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

    fn update_styles(&mut self) {
        let scale_2d_3d = self.transition_2d_3d_helper.scale_2d_3d();
        if scale_2d_3d > 0.0 && scale_2d_3d < 1.0 {
            self.renderer
                .api().update_style(StyleId::new("building_stand"), move |style| {
                // fyi, shift values to ensure a full opaque or transparent value
                let new_value = ((scale_2d_3d - 0.05) * 1.1).clamp(0.0, 1.0);
                style.set_alpha(new_value);
            });
        }
    }

    fn update_entities(&mut self) {
        let puck_location = self.current_world_position;
        let bearing = self.current_bearing;

        let cam_zoom = self.camera.scale() as f64;

        let cam_yaw = self.camera_controller.yaw;

        // SSAO is only enabled for near ground
        if unsafe { SSAO_ENABLED } {
            unsafe { SSAO_ENABLED = self.camera.scale() < 1.0 };
        }

        self.renderer
            .api() //  fyi, it seems to be fast enough(need to learn more here)
            .update_spatial_data("puck".to_string(), move |spatial_data| {
                spatial_data.scale = DVec3::splat(cam_zoom);
                let puck_location_offset = puck_location - spatial_data.transform;
                if puck_location_offset.length() >= Self::TELEPORT_THRESHOLD {
                    spatial_data.transform = puck_location;
                } else {
                    spatial_data.transform +=
                        (puck_location - spatial_data.transform) * Self::TEMP_ANIMATION_SPEED;
                }
                spatial_data.yaw +=
                    ((bearing - spatial_data.yaw) % 360.0) * Self::TEMP_ANIMATION_SPEED;
            });

        if self.should_animate() {
            let cam_pos = self.camera_controller.position;
            let cam_pos = DVec3::new(cam_pos.x, cam_pos.y, cam_pos.z);

            let transform_cam_offset = (self.current_world_position) - cam_pos;
            let transform_cam_offset_anim = transform_cam_offset * Self::TEMP_ANIMATION_SPEED * 2.0;
            let new_cam_pos = cam_pos + transform_cam_offset_anim;
            self.camera_controller.set_new_position(new_cam_pos);
        }

        if self.should_animate() || !self.cam_follow_mode {
            let new_cam_yaw = cam_yaw + ((self.camera_bearing - cam_yaw) % 360.0) * Self::TEMP_ANIMATION_SPEED;
            self.camera_controller.yaw = new_cam_yaw;
        }

        if self.should_animate() {
            self.camera_controller.pitch +=
                (self.current_pitch - self.camera_controller.pitch) * Self::TEMP_ANIMATION_SPEED;

            if let Some(zoom_lock) = self.cam_follow_zoom_lock {
                let current_dist = self.camera_controller.forward_len - zoom_lock;
                if current_dist > 0.0 {
                    let delta = 1.0 / (1.0 - (abs(current_dist) * 0.005 * Self::TEMP_ANIMATION_SPEED).min(0.05));
                    self.camera_controller.zoom_delta = delta;
                }
            }
        }
    }

    pub fn zoom_delta(&mut self, delta: f32, point: (f32, f32)) {
        self.reset_last_interaction();

        self.camera_controller.zoom_delta = delta as f64;

        if delta != 0.0 {
            let screen_center = self.screen_params.center();
            let diff = Vec2::from(point) - screen_center;

            let delta = delta as f64;
            let factor = 1.0 - (1.0 / delta);

            self.pan_delta((factor * diff.x as f64) as f32, (factor * diff.y as f64) as f32);
        }
    }

    pub fn pan_delta(&mut self, delta_x: f32, delta_y: f32) {
        self.reset_last_interaction();
        let ax = (delta_x / self.screen_params.width as f32) as f64;
        let ay = (delta_y / self.screen_params.height as f32) as f64;
        self.camera_controller.pan_delta = DVec2::new(self.world_width_on_screen * ax, self.world_height_on_screen * ay);
    }

    pub fn pitch_delta(&mut self, delta: f32) {
        self.reset_last_interaction();
        self.camera_controller.pitch = clamp(self.camera_controller.pitch + delta as f64, CameraController::MIN_PITCH, CameraController::MAX_PITCH);
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
            self.cam_follow_zoom_lock = Some(Self::ZOOM_LOCK_DIST);
            self.current_pitch = CameraController::MIN_PITCH;
            self.camera_bearing = self.current_bearing;
        } else {
            let new_bearing = Self::calc_nearest_bearing(0.0, self.camera_bearing);
            self.camera_bearing = new_bearing;
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
        let position = self.tiles_provider.lon_lat_to_world(&coord! {x: lon, y: lat}, MAX_ZOOM_LEVEL);
        self.current_world_position = DVec3::new(position.x, position.y, 0.0);

        if let Some(bearing) = bearing {
            let new_bearing = Self::calc_nearest_bearing(bearing as f64, self.current_bearing);
            self.current_bearing = new_bearing;
            if self.cam_follow_mode {
                self.camera_bearing = new_bearing;
            }
        }
    }

    fn calc_nearest_bearing(new_bearing: f64, prev_bearing: f64) -> f64 {
        let mut rot_diff = (new_bearing % 360.0) - (prev_bearing % 360.0);
        if rot_diff.abs() > 180.0 {
            rot_diff -= rot_diff.signum() * 360.0;
        }
        prev_bearing + rot_diff % 360.0
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
        );
    }

    fn create_location_coord_converter(&self) -> Box<dyn (Fn(&Point) -> Point) + Send> {
        let converter = self.tiles_provider.inner_converter();
        Box::new(move |p| {
            let coord: Coord<f64> = (p.x(), p.y()).into();
            let coord = converter.lon_lat_to_world(&coord, MAX_ZOOM_LEVEL);
            Point::new(coord.x, coord.y)
        })
    }

    fn load_styles(renderer_api: Arc<R::RAPI>) {
        spawn(move || {
            let mut styles = StyleLoader::load();
            if styles.is_empty() {
                error!("No styles loaded! Trying again!");
                sleep(Duration::from_millis(1000));
                styles = StyleLoader::load();
            }
            styles.into_iter().for_each(|style| {
                let style_id = StyleId::new(style.id);
                let actual_render_style = match style.render_style {
                    RenderStyle::Fill(color) => {
                        renderer_common::render_style::RenderStyle::fill(color.as_array())
                    }
                    RenderStyle::Border(color, percent) => {
                        renderer_common::render_style::RenderStyle::border(color.as_array(), percent)
                    }
                    RenderStyle::Dashed(color1, color2, dash_style) => {
                        let dash_style_value = match dash_style {
                            DashStyle::Solid => 0,
                            DashStyle::Circles => 1,
                        };
                        renderer_common::render_style::RenderStyle::dashed(
                            color1.as_array(),
                            color2.as_array(),
                            dash_style_value,
                        )
                    }
                };
                renderer_api
                    .update_style(style_id, move |style| *style = actual_render_style);
            });
        });
    }

    pub fn load_kml_path(&self, path_buf: PathBuf) {
        println!("Loading KML from {:?}", path_buf);
        let kml_group = KmlGroup::new(path_buf, self.create_location_coord_converter());

        // self.renderer.api().add_render_group(
        //     "kml_data".to_string(),
        //     SpatialData::transform(DVec3::new(0.0, 0.0, 0.0)),
        //     Box::new(kml_group),
        // );
    }

    pub fn clear_routes(&self) {
        self.route_controller
            .clear_routes(self.renderer.api());
    }

    #[allow(unused_variables)]
    fn start_sgnss_if_available(map_sender: Sender<MapEvent>) {
        #[cfg(feature = "sgnss")]
        start_sgnss(move |lat, lon| {
            map_sender.send(MapEvent::LatLon(lat, lon)).unwrap();
        });
    }

    pub fn update_tile_store<F>(&mut self, block: F)
    where
        F: FnOnce(&mut T),
    {
        let prev_world_coord = self.current_world_position.truncate();
        let prev_lon_lat = self.tiles_provider.world_to_lon_lat(&coord! { x: prev_world_coord.x, y: prev_world_coord.y}, MAX_ZOOM_LEVEL);

        block(&mut self.tiles_provider);

        let new_world_coord = self.tiles_provider.lon_lat_to_world(&prev_lon_lat, MAX_ZOOM_LEVEL);
        let new_world_coord = DVec2::new(new_world_coord.x, new_world_coord.y);
        let world_offset = new_world_coord - prev_world_coord;
        self.camera.global_offset(world_offset);
        self.camera_controller.position += world_offset.extend(0.0);
        self.set_lon_lat_bearing(prev_lon_lat.x, prev_lon_lat.y, Some(self.current_bearing as f32));
    }
}
