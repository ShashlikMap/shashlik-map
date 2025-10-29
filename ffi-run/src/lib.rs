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

    fn temp_external_input(&self, pressed: bool) {
        let mut shashlik_map = self.shashlik_map.write().unwrap();
        shashlik_map.temp_external_input(pressed);
    }
}

uniffi::setup_scaffolding!();
