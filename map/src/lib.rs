extern crate core;

use crate::style_loader::StyleLoader;
use crate::test_puck_group::TestSimplePuck;
use crate::test_simple_path_group::TestSimplePathGroup;
use crate::tiles::mesh_loader::MeshLoader;
use crate::tiles::tile_data::TileData;
use crate::tiles::tiles_provider::TilesProvider;
use cgmath::Vector3;
use futures::executor::block_on;
use futures::{pin_mut, Stream, StreamExt};
use geo_types::private_utils::get_bounding_rect;
use geo_types::{coord, Coord, Rect};
use geo_types::{LineString, Polygon};
use renderer::camera::CameraController;
use renderer::canvas_api::CanvasApi;
use renderer::modifier::render_modifier::SpatialData;
use renderer::render_group::RenderGroup;
use renderer::renderer_api::RendererApi;
use renderer::styles::style_id::StyleId;
use renderer::{Renderer, ShashlikRenderer};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::spawn;
use wgpu_canvas::wgpu_canvas::WgpuCanvas;

mod style_loader;
mod test_puck_group;
mod test_simple_path_group;
pub mod tiles;

pub struct ShashlikMap<T: TilesProvider> {
    renderer: Box<ShashlikRenderer>,
    camera_controller: Rc<RefCell<CameraController>>,
    tiles_provider: T,
    last_area_latlon: Rect,
    camera_offset: Vector3<f32>,
    style_loader: StyleLoader,
    pub temp_color: f32,
}

impl RenderGroup for TileData {
    fn content(&self, canvas: &mut CanvasApi) {
        self.geometry_data.iter().for_each(|data| {
            canvas.geometry_data(data);
        });
    }
}

impl<T: TilesProvider> ShashlikMap<T> {
    pub async fn new(
        canvas: Box<dyn WgpuCanvas>,
        tiles_provider: T,
    ) -> anyhow::Result<ShashlikMap<T>> {
        let camera_controller = Rc::new(RefCell::new(CameraController::new(1.0)));
        Self::new_with_camera_controller_internal(camera_controller, canvas, tiles_provider).await
    }
    #[cfg(all(target_os = "macos"))]
    pub async fn new_with_camera_controller(
        camera_controller: Rc<RefCell<CameraController>>,
        canvas: Box<dyn WgpuCanvas>,
        tiles_provider: T,
    ) -> anyhow::Result<ShashlikMap<T>> {
        Self::new_with_camera_controller_internal(camera_controller, canvas, tiles_provider).await
    }

    pub async fn new_with_camera_controller_internal(
        camera_controller: Rc<RefCell<CameraController>>,
        canvas: Box<dyn WgpuCanvas>,
        mut tiles_provider: T,
    ) -> anyhow::Result<ShashlikMap<T>> {
        let renderer = ShashlikRenderer::new(camera_controller.clone(), canvas).await?;
        let tiles_stream = tiles_provider.tiles();

        let initial_coord: Coord<f64> = (139.757080078125, 35.68798828125).into();
        let camera_offset = T::lat_lon_to_world(&initial_coord);

        let camera_offset = (camera_offset.x, camera_offset.y, 0.0).into();

        let mut puck_spatial_data = SpatialData::transform(Vector3::new(0.0, 0.0, 0.0));
        puck_spatial_data.scale(1.0);
        renderer.api.add_render_group(
            "puck".to_string(),
            1,
            puck_spatial_data,
            Box::new(TestSimplePuck {}),
        );

        renderer.api.add_render_group(
            "some_icon".to_string(),
            0,
            SpatialData::transform(Vector3::new(30.0, 0.0, 0.0)),
            Box::new(TestSimplePathGroup {
                path: MeshLoader::load_test_line_path(),
                style_id: StyleId("simple_icon_style"),
            }),
        );

        Self::run_tiles(renderer.api.clone(), tiles_stream, camera_offset);
        let map = ShashlikMap {
            renderer: Box::new(renderer),
            camera_controller,
            tiles_provider,
            last_area_latlon: Rect::new((0.0, 0.0), (0.0, 0.0)),
            camera_offset: camera_offset.cast().unwrap(),
            style_loader: StyleLoader::new(),
            temp_color: 0.0,
        };
        map.temp_on_load_styles();
        Ok(map)
    }

    pub fn clip_to_latlon(&self, coord: &Coord<f64>) -> Option<Coord<f64>> {
        let world_on_ground = self.camera_controller.borrow().clip_to_world(coord)?;
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
        tiles_stream: impl Stream<Item = (Option<TileData>, HashSet<String>)> + Send + 'static,
        camera_offset: Vector3<f64>,
    ) {
        spawn(move || {
            block_on(async {
                pin_mut!(tiles_stream);
                loop {
                    let item = tiles_stream.next().await;
                    match item {
                        None => break,
                        Some((item, to_remove)) => {
                            if let Some(item) = item {
                                renderer_api.add_render_group(
                                    item.key.to_string(),
                                    0,
                                    SpatialData::transform(
                                        item.position - camera_offset.cast().unwrap(),
                                    ).size(item.size),
                                    Box::new(item),
                                );
                            }

                            if !to_remove.is_empty() {
                                renderer_api.clear_render_groups(to_remove);
                            }
                        }
                    }
                }
            })
        });
    }

    pub fn update_and_render(&mut self) {
        self.temp_update_some_styles();

        self.renderer.update();

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

        self.tiles_provider.abc(zoom_level);
        if area_latlon != self.last_area_latlon {
            self.tiles_provider.load(area_latlon, zoom_level);
        }

        self.last_area_latlon = area_latlon;
    }

    pub fn renderer(&mut self) -> &mut dyn Renderer {
        self.renderer.as_mut()
    }

    fn temp_update_some_styles(&mut self) {
        self.renderer
            .api
            .update_spatial_data("some_icon".to_string(), |spatial_data| {
                spatial_data.transform.y += 0.02;
            });

        let cam_zoom = -self.camera_controller.borrow().camera_z / 100.0;
        self.renderer
            .api
            .update_spatial_data("puck".to_string(), move |spatial_data| {
                spatial_data.scale = cam_zoom as f64;
            });
    }

    pub fn zoom_delta(&self, delta: f32) {
        self.camera_controller.borrow_mut().zoom_delta = delta;
    }

    pub fn pan_delta(&self, delta_x: f32, delta_y: f32) {
        self.camera_controller.borrow_mut().pan_delta = (delta_x, delta_y);
    }
    
    pub fn temp_on_load_styles(&self) {
        self.style_loader.load(self.renderer.api.clone());
    }
}
