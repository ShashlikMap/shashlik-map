mod platform;

use map::ShashlikMap;
use std::sync::RwLock;
use map::tiles::old_tiles_provider::OldTilesProvider;
// use old_tiles_gen::source::tiles_sqlite_store::TilesSQLiteStore;
use old_tiles_gen::source::reqwest_source::ReqwestSource;

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

    fn zoom_delta(&self, delta: f32) {
        let shashlik_map = self.shashlik_map.read().unwrap();
        shashlik_map.zoom_delta(delta);
    }

    fn pan_delta(&self, delta_x: f32, delta_y: f32) {
        let shashlik_map = self.shashlik_map.read().unwrap();
        shashlik_map.pan_delta(delta_x, delta_y);
    }

    fn set_lat_lon(&self, lat: f64, lon: f64) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        shashlik_map.set_lat_lon(lat, lon);
    }
}

uniffi::setup_scaffolding!();
