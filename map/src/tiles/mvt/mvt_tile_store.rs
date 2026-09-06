use std::env;
use crate::tiles::mvt::mvt_parser::MvtParser;
use crate::tiles::tiles_provider::{MercatorConverter, MercatorProvider, TilesProviderStore};
use log::error;
use osm::map::{MapGeomObject, MapGeometry};
use osm::tiles::TileKey;
use reqwest::header::{HeaderMap, HeaderValue, ORIGIN};
use std::time::{Duration, SystemTime};
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest_middleware::ClientWithMiddleware;
use tokio::runtime::Runtime;

const HTTP_CACHE_ENABLED: bool = true;

pub struct MvtTileStore {
    tokio_rt: Runtime,
    mvt_parser: MvtParser,
    client: ClientWithMiddleware,
}


impl MvtTileStore {
    pub fn new() -> Self {
        let tokio_rt = Runtime::new().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, HeaderValue::from_static("shashlikmap.com"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        let client_builder = if !HTTP_CACHE_ENABLED || cfg!(target_os = "android") || cfg!(target_os = "ios") {
            // fyi, We don't use http cache on mobile device at this moment
            // It requires to pass a files/cache native folder
            reqwest_middleware::ClientBuilder::new(client)
        } else {
            let mut cache_dir = env::current_exe().expect("Failed to get current executable path");
            cache_dir.pop();
            cache_dir.push("maptiler-http-cache");
            reqwest_middleware::ClientBuilder::new(client)
                .with(Cache(HttpCache {
                    mode: CacheMode::Default,
                    manager: CACacheManager::new(cache_dir, false),
                    options: HttpCacheOptions::default(),
                }))
        };

        let client = client_builder.build();

        Self {
            tokio_rt,
            mvt_parser: MvtParser::default(),
            client,
        }
    }

    async fn fetch_tile_inner(&self, x: i32, y: i32, z: i32) -> Result<Vec<u8>, reqwest_middleware::Error> {
        let api_key = option_env!("MAPTILER_API_KEY").expect("MAPTILER_API_KEY should be set");
        let bytes = self
            .client
            .get(format!(
                "https://api.maptiler.com/tiles/v4/{z}/{x}/{y}.pbf?key={api_key}"
            ))
            .send().await?.error_for_status()?.bytes().await?.to_vec();
        Ok(bytes)
    }

    fn fetch_tile(&self, x: i32, y: i32, z: i32) -> Result<Vec<u8>, reqwest_middleware::Error> {
        let t1 = SystemTime::now();
        let bytes = self.tokio_rt.block_on(self.fetch_tile_inner(x, y, z))?;
        error!(
            "get_map_tiler_tile, x = {}, y = {}, z = {}, total_time = {:?}, len = {}",
            x,
            y,
            z,
            t1.elapsed(),
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
