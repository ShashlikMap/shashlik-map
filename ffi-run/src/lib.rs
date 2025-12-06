uniffi::setup_scaffolding!();

mod platform;

use map::ShashlikMap;
use std::sync::RwLock;
use osm::source::reqwest_source::ReqwestSource;
use map::tiles::old_tiles_provider::OldTilesProvider;


#[derive(uniffi::Object)]
pub struct ShashlikMapApi {
    // TODO ?Can't use generic for FFI ShashlikMapApi?
    shashlik_map: RwLock<ShashlikMap<OldTilesProvider<ReqwestSource>>>,
}

unsafe impl Sync for ShashlikMapApi {}
unsafe impl Send for ShashlikMapApi {}

#[uniffi::export]
impl ShashlikMapApi {
    fn render(&self) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        // TODO handle result
        shashlik_map.update_and_render();
    }

    fn zoom_delta(&self, delta: f32, point_x: f32, point_y: f32) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        shashlik_map.zoom_delta(delta, (point_x, point_y));
    }

    fn pan_delta(&self, delta_x: f32, delta_y: f32) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        shashlik_map.pan_delta(delta_x, delta_y);
    }

    fn set_lat_lon_bearing(&self, lat: f64, lon: f64, bearing: Option<f32>) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        shashlik_map.set_lat_lon_bearing(lat, lon, bearing);
    }

    fn set_cam_follow_mode(&self, enabled: bool) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        shashlik_map.cam_follow_mode = enabled;
    }
}
