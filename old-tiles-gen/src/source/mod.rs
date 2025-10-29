pub mod reqwest_source;
pub mod tiles_sqlite_store;

use crate::source::reqwest_source::ReqwestSource;
use crate::source::tiles_sqlite_store::TilesSQLiteStore;
use error_stack::{Report, ResultExt};
use thiserror::Error;

pub trait TileSource: Send + Sync + 'static {
    fn fetch(&self, x: i32, y: i32, z: i32) -> Result<Vec<u8>, Report<TileSourceFetchError>>;
}

#[derive(Debug, Error)]
pub enum TileSourceFetchError {
    #[error("Internal")]
    Internal,
}

impl TileSource for TilesSQLiteStore {
    fn fetch(&self, x: i32, y: i32, z: i32) -> Result<Vec<u8>, Report<TileSourceFetchError>> {
        self.get_tile(x, y, z)
            .change_context(TileSourceFetchError::Internal)
    }
}

impl TileSource for ReqwestSource {
    fn fetch(&self, x: i32, y: i32, z: i32) -> Result<Vec<u8>, Report<TileSourceFetchError>> {
        let temp_data = self
            .get_tile(x, y, z)
            .change_context(TileSourceFetchError::Internal);
        temp_data
    }
}
