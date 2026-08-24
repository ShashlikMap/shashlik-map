use crate::tiles::mvt::mvt_parser::MvtParser;
use crate::tiles::tiles_provider::{MercatorConverter, MercatorProvider, TilesProviderStore};
use log::error;
use osm::map::{MapGeomObject, MapGeometry};
use osm::tiles::TileKey;
use reqwest::header::{HeaderMap, HeaderValue, ORIGIN};
use std::time::{Duration, SystemTime};

pub struct MvtTileStore {
    mvt_parser: MvtParser,
    client: reqwest::blocking::Client,
}

impl MvtTileStore {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, HeaderValue::from_static("shashlikmap.com"));
        let client = reqwest::blocking::Client::builder()
            .default_headers(headers)
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        Self {
            mvt_parser: MvtParser::default(),
            client,
        }
    }

    fn fetch_tile(&self, x: i32, y: i32, z: i32) -> Result<Vec<u8>, reqwest::Error> {
        let t1 = SystemTime::now();
        let api_key = option_env!("MAPTILER_API_KEY").expect("MAPTILER_API_KEY should be set");
        let response = self
            .client
            .get(format!(
                "https://api.maptiler.com/tiles/v4/{z}/{x}/{y}.pbf?key={api_key}"
            ))
            .send()?
            .error_for_status();
        let bytes_res = response.and_then(|response| response.bytes())?;
        let bytes = bytes_res.to_vec();
        let t2 = SystemTime::now();
        error!(
            "get_map_tiler_tile, x = {}, y = {}, z = {}, total_time = {:?}, len = {}",
            x,
            y,
            z,
            t2.duration_since(t1),
            bytes.len()
        );
        Ok(bytes)
    }
}

impl MercatorProvider for MvtTileStore {}
impl MercatorConverter for MvtTileStore {}

impl TilesProviderStore for MvtTileStore {
    fn load(&self, tile_key: &TileKey) -> Vec<(MapGeomObject, MapGeometry<f32>)> {
        let data = self
            .fetch_tile(tile_key.tile_x, tile_key.tile_y, tile_key.zoom_level)
            .unwrap_or_default();
        self.mvt_parser
            .read_mvt_tile(data.as_slice(), tile_key)
            .unwrap_or_default()
    }
}
