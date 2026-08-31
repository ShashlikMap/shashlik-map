uniffi::setup_scaffolding!();

mod platform;

use map::feature_processor::ShashlikFeatureProcessor;
use map::tiles::default_tiles_provider::DefaultTilesProvider;
use map::ShashlikMap;
use renderer_gpu::GpuRenderer;
use renderer_common::{PreviewType, TilesType};
use std::sync::RwLock;
use log::__private_api::enabled;

#[derive(uniffi::Object)]
pub struct ShashlikMapApi {
    // TODO ?Can't use generic for FFI ShashlikMapApi?
    shashlik_map: RwLock<ShashlikMap<GpuRenderer, DefaultTilesProvider<ShashlikFeatureProcessor>>>,
}

unsafe impl Sync for ShashlikMapApi {}
unsafe impl Send for ShashlikMapApi {}

#[derive(uniffi::Record)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

#[derive(uniffi::Enum)]
pub enum RouteCosting {
    Auto, Pedestrian, Motorbike
}

impl From<RouteCosting> for map::route::RouteCosting {
    fn from(value: RouteCosting) -> Self {
        match value {
            RouteCosting::Auto => map::route::RouteCosting::Auto,
            RouteCosting::Pedestrian => map::route::RouteCosting::Pedestrian,
            RouteCosting::Motorbike => map::route::RouteCosting::Motorbike
        }
    }
}

#[uniffi::export]
impl ShashlikMapApi {
    fn render(&self) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        // TODO handle result
        shashlik_map.update_and_render(());
    }

    fn resize(&self, width: u32, height: u32) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        shashlik_map.resize(width, height);
    }

    fn zoom_delta(&self, delta: f32, point_x: f32, point_y: f32) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        shashlik_map.zoom_delta(delta, (point_x, point_y));
    }

    fn pan_delta(&self, delta_x: f32, delta_y: f32) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        shashlik_map.pan_delta(delta_x, delta_y);
    }

    fn pitch_delta(&self, delta: f32) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        shashlik_map.pitch_delta(delta);
    }

    fn set_lat_lon_bearing(&self, lat: f64, lon: f64, bearing: Option<f32>) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        // swap lat/lon to lon/lat
        shashlik_map.set_lon_lat_bearing(lon, lat, bearing);
    }

    fn set_cam_follow_mode(&self, enabled: bool) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        shashlik_map.set_camera_follow_mode(enabled);
    }

    fn set_ssao_mode(&self, enabled: bool) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        shashlik_map.renderer.update_config(|config| {
            config.ssao_enabled = enabled;
        });
    }

    fn set_preview_enabled(&self, enabled: bool) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        shashlik_map.renderer.update_config(|config| {
            if enabled {
                config.preview_type = PreviewType::Camera;
            } else {
                config.preview_type = PreviewType::None;
            }
        });
    }

    fn set_mvt_tileset(&self, enabled: bool) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        shashlik_map.update_tile_store(|tile_store| {
            let tiles_type = if enabled {
                TilesType::MapTiler
            } else {
                TilesType::V0
            };
            tile_store.set_tiles_type(tiles_type);
        });
    }

    fn calculate_route_to_lat_lon(&self, lat: f64, lon: f64, route_costing: RouteCosting) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        // swap lat/lon to lon/lat
        shashlik_map.create_route_to((lon, lat), route_costing.into());
    }

    fn calculate_route(&self, point_x: f32, point_y: f32, route_costing: RouteCosting) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        shashlik_map.create_route_to_screen_point(point_x, point_y, route_costing.into());
    }

    fn draw_track(&self, points: Vec<LatLon>) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        let lon_lats: Vec<(f64, f64)> = points.iter().map(|p| (p.lon, p.lat)).collect();
        shashlik_map.draw_track(lon_lats);
    }

    fn clear_track(&self) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        shashlik_map.clear_track();
    }
}
