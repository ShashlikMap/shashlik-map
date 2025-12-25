uniffi::setup_scaffolding!();

mod platform;

use map::tiles::shashlik_tiles_provider_v0::ShashlikTilesProviderV0;
use map::ShashlikMap;
use osm::source::reqwest_source::ReqwestSource;
use std::sync::RwLock;
use map::feature_processor::ShashlikFeatureProcessor;

#[derive(uniffi::Object)]
pub struct ShashlikMapApi {
    // TODO ?Can't use generic for FFI ShashlikMapApi?
    shashlik_map: RwLock<ShashlikMap<ShashlikTilesProviderV0<ReqwestSource, ShashlikFeatureProcessor>>>,
}

unsafe impl Sync for ShashlikMapApi {}
unsafe impl Send for ShashlikMapApi {}

#[derive(uniffi::Enum)]
pub enum RouteCosting {
    Pedestrian, Motorbike
}

impl From<RouteCosting> for map::route::RouteCosting {
    fn from(value: RouteCosting) -> Self {
        match value {
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
        shashlik_map.update_and_render();
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
        shashlik_map.set_lat_lon_bearing(lat, lon, bearing);
    }

    fn set_cam_follow_mode(&self, enabled: bool) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        shashlik_map.set_camera_follow_mode(enabled);
    }

    fn calculate_route_to_lat_lon(&self, lat: f64, lon: f64, route_costing: RouteCosting) {
        let shashlik_map = self.shashlik_map.read().unwrap();
        shashlik_map.create_route_to((lat, lon), route_costing.into());
    }

    fn calculate_route(&self, point_x: f32, point_y: f32, route_costing: RouteCosting) {
        let shashlik_map = self.shashlik_map.read().unwrap();
        shashlik_map.create_route_to_screen_point(point_x, point_y, route_costing.into());
    }
}
