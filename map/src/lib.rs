extern crate core;

use crate::style_loader::StyleLoader;
use crate::test_kml_viewer_group::TestKmlGroup;
use crate::test_puck_group::TestSimplePuck;
use crate::tiles::tile_data::TileData;
use crate::tiles::tiles_provider::{TilesMessage, TilesProvider};
use cgmath::{Matrix4, SquareMatrix, Vector2, Vector3};
use futures::executor::block_on;
use futures::{pin_mut, Stream, StreamExt};
use geo_types::private_utils::get_bounding_rect;
use geo_types::{coord, Coord, Point, Rect};
use geo_types::{LineString, Polygon};
use renderer::canvas_api::CanvasApi;
use renderer::modifier::render_modifier::SpatialData;
use renderer::render_group::RenderGroup;
use renderer::renderer_api::RendererApi;
use renderer::{Renderer, ShashlikRenderer};
use std::cell::RefCell;
use std::mem;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::spawn;
use wgpu_canvas::wgpu_canvas::WgpuCanvas;
use crate::camera::{Camera, CameraController};

mod style_loader;
mod test_puck_group;
pub mod tiles;
mod test_kml_viewer_group;
mod camera;

pub struct ShashlikMap<T: TilesProvider> {
    renderer: Box<ShashlikRenderer>,
    camera: Camera,
    camera_controller: Rc<RefCell<CameraController>>,
    tiles_provider: T,
    last_area_latlon: Rect,
    camera_offset: Vector3<f32>,
    current_lat_lon: Vector3<f32>,
    current_bearing: f32,
    style_loader: StyleLoader,
    pub temp_color: f32,
    pub cam_follow_mode: bool,
    screen_size: (f32, f32),
}

impl RenderGroup for TileData {
    fn content(&mut self, canvas: &mut CanvasApi) {
        mem::take(&mut self.geometry_data).into_iter().for_each(|data| {
            canvas.geometry_data(data);
        });
    }
}

impl<T: TilesProvider> ShashlikMap<T> {

    const TEMP_ANIMATION_SPEED: f32 = 0.03;
    pub async fn new(
        canvas: Box<dyn WgpuCanvas>,
        mut tiles_provider: T,
    ) -> anyhow::Result<ShashlikMap<T>> {
        let camera_controller = Rc::new(RefCell::new(CameraController::new(1.0)));
        // TODO support resize
        let screen_size = (canvas.config().width as f32, canvas.config().height as f32);

        let mut camera = Camera {
            eye: (0.0, 0.0, 200.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: screen_size.0 / screen_size.1,
            fovy: 45.0,
            znear: 1.0,
            zfar: 2000000.0,
            perspective_matrix: Matrix4::identity(),
            inv_view_proj_matrix: Matrix4::identity(),
        };
        // FIXME Android should call resize by itself!
        camera.resize();


        let renderer = ShashlikRenderer::new(canvas).await?;
        let tiles_stream = tiles_provider.tiles();

        let initial_coord: Coord<f64> = (139.757080078125, 35.68798828125).into();
        let camera_offset = T::lat_lon_to_world(&initial_coord);

        let camera_offset: Vector3<f64> = (camera_offset.x, camera_offset.y, 0.0).into();

        let mut puck_spatial_data = SpatialData::transform(Vector3::new(0.0, 0.0, 0.0));
        puck_spatial_data.scale(1.0);
        renderer.api.add_render_group(
            "puck".to_string(),
            0,
            puck_spatial_data,
            Box::new(TestSimplePuck {}),
        );

        Self::run_tiles(renderer.api.clone(), tiles_stream, camera_offset);
        let map = ShashlikMap {
            renderer: Box::new(renderer),
            camera,
            camera_controller,
            tiles_provider,
            last_area_latlon: Rect::new((0.0, 0.0), (0.0, 0.0)),
            current_lat_lon: camera_offset.cast().unwrap(),
            current_bearing: 0.0,
            camera_offset: camera_offset.cast().unwrap(),
            style_loader: StyleLoader::new(),
            temp_color: 0.0,
            cam_follow_mode: true,
            screen_size,
        };
        map.load_styles();
        Ok(map)
    }

    pub fn clip_to_latlon(&self, coord: &Coord<f64>) -> Option<Coord<f64>> {
        let world_on_ground = self.camera.clip_to_world(coord)?;
        Some(T::world_to_lat_lon(
            &(
                world_on_ground.x + self.camera_offset.x as f64,
                world_on_ground.y + self.camera_offset.y as f64,
            )
                .into(),
        ))
    }

    fn run_tiles(
        renderer_api: Arc<RendererApi>,
        tiles_stream: impl Stream<Item = TilesMessage> + Send + 'static,
        camera_offset: Vector3<f64>,
    ) {
        spawn(move || {
            block_on(async {
                pin_mut!(tiles_stream);
                loop {
                    let item = tiles_stream.next().await;
                    match item {
                        None => break,
                        Some(msg) => {
                            match msg {
                                TilesMessage::TilesData(data) => {
                                    data.into_iter().for_each(|item| {
                                        renderer_api.add_render_group(
                                            item.key.to_string(),
                                            0,
                                            SpatialData::transform(
                                                item.position - camera_offset.cast().unwrap(),
                                            ).size(item.size),
                                            Box::new(item),
                                        );
                                    });
                                }
                                TilesMessage::ToRemove(set) => {
                                    renderer_api.clear_render_groups(set);
                                }
                            }
                        }
                    }
                }
            })
        });
    }

    pub fn update_and_render(&mut self) {
        self.camera_controller.borrow_mut().update_camera(&mut self.camera);

        self.update_entities();

        self.renderer.update(self.camera.build_view_projection_matrix());

        self.fetch_tiles();

        self.renderer.render().unwrap();
    }

    fn fetch_tiles(&mut self) {
        let zoom_level = self.camera_controller.borrow().camera_z / 100.0;
        let zoom_level = (zoom_level.log2().round() as i32).max(0);
        let p1 = self.clip_to_latlon(&coord! {x: -1.0, y: -1.0}).unwrap();
        let p2 = self.clip_to_latlon(&coord! {x: 1.0, y: -1.0}).unwrap();
        let p3 = self.clip_to_latlon(&coord! {x: 1.0, y: 1.0}).unwrap();
        let p4 = self.clip_to_latlon(&coord! {x: -1.0, y: 1.0}).unwrap();

        let poly: Polygon<f64> = Polygon::new(LineString(vec![p1, p2, p4, p3]), Vec::new());
        let area_latlon = get_bounding_rect(poly.exterior()).unwrap();

        // if area_latlon != self.last_area_latlon {
            self.tiles_provider.load(area_latlon, zoom_level);
        // }

        self.last_area_latlon = area_latlon;
    }

    pub fn renderer(&mut self) -> &mut dyn Renderer {
        self.renderer.as_mut()
    }

    fn update_entities(&mut self) {
        let puck_location = self.current_lat_lon - self.camera_offset.cast().unwrap();
        let bearing = self.current_bearing;
        let cam_follow_mode =  self.cam_follow_mode;
        self.renderer
            .api
            .update_spatial_data("puck".to_string(), move |spatial_data| {
                spatial_data.scale = 15.0;
                spatial_data.transform += (puck_location.cast().unwrap() - spatial_data.transform) * Self::TEMP_ANIMATION_SPEED as f64;
                // TODO Puck should be aligned with the map plane too
                if !cam_follow_mode {
                    spatial_data.rotation += ((-bearing - spatial_data.rotation) % 360.0) * Self::TEMP_ANIMATION_SPEED;
                } else {
                    spatial_data.rotation = (spatial_data.rotation % 360.0) * (1.0f32 - Self::TEMP_ANIMATION_SPEED) % 360.0;
                }
            });

        let cam_rotation = self.camera_controller.borrow_mut().rotation;
        let new_cam_rotation = if self.cam_follow_mode {
            let cam_pos = self.camera_controller.borrow().position;
            let cam_pos = Vector3::new(cam_pos.x, cam_pos.y, cam_pos.z);
            let new_cam_pos = cam_pos + ((self.current_lat_lon - self.camera_offset) - cam_pos) * Self::TEMP_ANIMATION_SPEED;
            self.camera_controller.borrow_mut().set_new_position(new_cam_pos);

            cam_rotation + ((self.current_bearing - cam_rotation) % 360.0) * Self::TEMP_ANIMATION_SPEED
        } else {
            cam_rotation * (1.0f32 - Self::TEMP_ANIMATION_SPEED) % 360.0
        };
        self.camera_controller.borrow_mut().rotation = new_cam_rotation;
    }

    pub fn zoom_delta(&self, delta: f32, point: (f32, f32)) {
        self.camera_controller.borrow_mut().zoom_delta = delta;

        let ratio = self.screen_size.0 / self.screen_size.1;
        let half_screen_size = Vector2::from(self.screen_size) * 0.5f32;
        let diff = (Vector2::from(point) - half_screen_size) * 0.5f32;
        let px = diff.x / half_screen_size.x;
        let py = diff.y / half_screen_size.y;
        self.pan_delta(delta * px * ratio, delta * py);
    }

    pub fn pan_delta(&self, delta_x: f32, delta_y: f32) {
        // pan is disabled for now
        if !self.cam_follow_mode {
            self.camera_controller.borrow_mut().pan_delta = (delta_x, delta_y);
        }
    }

    pub fn set_lat_lon_bearing(&mut self, lat: f64, lon: f64, bearing: Option<f32>) {
        let position = T::lat_lon_to_world(&coord! {x: lon, y: lat});
        self.current_lat_lon = Vector3::new(position.x as f32, position.y as f32, 0.0);
        if let Some(bearing) = bearing {
            let mut rot_diff = (bearing % 360.0) - (self.current_bearing % 360.0);
            if rot_diff.abs() > 180.0 {
                rot_diff -= rot_diff.signum() * 360.0;
            }
            self.current_bearing += rot_diff % 360.0;
        }
    }

    fn load_styles(&self) {
        self.style_loader.load(self.renderer.api.clone());
    }

    pub fn load_kml_path(&self, path_buf: PathBuf) {
        println!("Loading KML from {:?}", path_buf);
        let camera_offset = self.camera_offset;
        self.renderer.api.add_render_group(
            "kml_data".to_string(),
            0,
            SpatialData::transform(Vector3::new(0.0, 0.0, 0.0)),
            Box::new(TestKmlGroup::new(path_buf, Box::new(move |p| {
                let coord: Coord<f64> = (p.x(), p.y()).into();
                let coord = T::lat_lon_to_world(&coord);
                Point::new(coord.x - camera_offset.x as f64, coord.y - camera_offset.y as f64)
            }))),
        );
    }
}
