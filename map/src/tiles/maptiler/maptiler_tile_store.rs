use crate::tiles::parsers::mvt_parser::MvtParser;
use crate::tiles::tiles_provider::{MercatorConverter, MercatorProvider, TilesProviderStore};
use crate::tiles::{ShashlikMapGeomObject, ShashlikMapGeomObjectKind, ShashlikMapLinearRef, ShashlikMapPointObjectKind};
use geo::{Distance, Euclidean, LineLocatePoint};
use geo_types::Point;
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use log::error;
use osm::map::MapGeometry;
use osm::tiles::TileKey;
use reqwest::header::{HeaderMap, HeaderValue, ORIGIN};
use reqwest_middleware::ClientWithMiddleware;
use std::env;
use std::time::{Duration, SystemTime};
use tokio::runtime::Runtime;
use crate::tiles::parsers::mvt_scheme_parser::MvtSchemeParser;

const HTTP_CACHE_ENABLED: bool = true;

pub struct MaptilerTileStore {
    tokio_rt: Runtime,
    mvt_parser: MvtParser,
    client: ClientWithMiddleware,
}


impl MaptilerTileStore {
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
            // fyi, keep in mind that the folder might be read-only. Let's ignore it for now.
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
            mvt_parser: MvtParser::new(MvtSchemeParser::new_map_tiler_v4()),
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

impl MercatorProvider for MaptilerTileStore {}
impl MercatorConverter for MaptilerTileStore {}

impl TilesProviderStore for MaptilerTileStore {
    fn load(&self, tile_key: &TileKey) -> Vec<(ShashlikMapGeomObject, MapGeometry<f32>)> {
        let data = self
            .fetch_tile(tile_key.tile_x, tile_key.tile_y, tile_key.zoom_level)
            .unwrap_or_default();
        let mut data = self.mvt_parser
            .read_mvt_tile(data.as_slice(), tile_key)
            .unwrap_or_default();

        if tile_key.zoom_level < 15 {
            return data
        }

        // TODO This is temporary solution since MapTiler doesn't have LinearRefs for roads/lines.
        // Only for 15 zoom level
        let crossing_points: Vec<_> = data.iter().filter_map(|(obj, geom)| {
            match &obj.kind {
                ShashlikMapGeomObjectKind::Poi(data) => {
                    match data.kind {
                        ShashlikMapPointObjectKind::Crossing => {
                            Some(geom.coord().clone())
                        }
                        _ => None
                    }
                }
                _ => None
            }
        }).collect();

        if crossing_points.is_empty() {
            return data;
        }

        // TODO This is temporary solution to find linear refs and inject them into ways
        data.iter_mut().for_each(|(obj, geom)| {
            match &mut obj.kind {
                ShashlikMapGeomObjectKind::Way(info) => {
                    info.linear_refs = crossing_points.iter().filter_map(|q| {
                        let line = geom.line_string();
                        let dist_to_point = Euclidean.distance(line, &Point::from(*q));
                        // this is simple condition since, we don't care about precision and the fact the some marks will be missing for POC
                        if dist_to_point == 0.0 && info.layer >= 0 {
                            if let Some(linear_ref) = line.line_locate_point(&Point::from(*q)) {
                                Some(ShashlikMapLinearRef::Crossing(linear_ref))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }).collect();
                }
                _ => {}
            }
        });
        data
    }
}
