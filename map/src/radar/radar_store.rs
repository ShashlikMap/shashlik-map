use fast_mvt::serde_json;
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use log::error;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest_middleware::ClientWithMiddleware;
use serde::Deserialize;
use std::env;
use std::time::Duration;
use tokio::runtime::Runtime;

const BASE_URL: &str = "https://api.rainbow.ai/tiles/v1";
const AUTH_HEADER: &str = "Ocp-Apim-Subscription-Key";
const HTTP_CACHE_ENABLED: bool = true;

#[derive(Deserialize)]
struct SnapshotResponse {
    snapshot: u64,
}

/// Thin HTTP client for rainbow.ai radar tiles.
/// https://doc.rainbow.ai/api-ref/scalar?spec=tiles
pub(crate) struct RadarStore {
    tokio_rt: Runtime,
    // Snapshot endpoint changes every few minutes, so it must bypass the HTTP cache
    snapshot_client: reqwest::Client,
    // Tiles are immutable per snapshot ("cache-control: immutable"), so they can be cached
    tile_client: ClientWithMiddleware,
}

impl RadarStore {
    pub fn new(api_key: &str) -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();
        let mut key = HeaderValue::from_str(api_key)?;
        key.set_sensitive(true);
        headers.insert(AUTH_HEADER, key);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .tcp_keepalive(Duration::from_secs(30))
            .timeout(Duration::from_secs(10))
            .build()?;

        let tile_client = if !HTTP_CACHE_ENABLED || cfg!(target_os = "android") || cfg!(target_os = "ios") {
            reqwest_middleware::ClientBuilder::new(client.clone())
        } else {
            let mut cache_dir = env::current_exe()?;
            cache_dir.pop();
            cache_dir.push("rainbow-http-cache");
            reqwest_middleware::ClientBuilder::new(client.clone())
                .with(Cache(HttpCache {
                    mode: CacheMode::Default,
                    manager: CACacheManager::new(cache_dir, false),
                    options: HttpCacheOptions::default(),
                }))
        }
        .build();

        Ok(Self {
            tokio_rt: Runtime::new()?,
            snapshot_client: client,
            tile_client,
        })
    }

    /// Latest available radar snapshot (unix timestamp, seconds).
    pub fn latest_snapshot(&self) -> Option<u64> {
        let result: anyhow::Result<u64> = self.tokio_rt.block_on(async {
            let bytes = self
                .snapshot_client
                .get(format!("{BASE_URL}/snapshot?layer=radars"))
                .send().await?.error_for_status()?.bytes().await?;
            Ok(serde_json::from_slice::<SnapshotResponse>(&bytes)?.snapshot)
        });
        result.inspect_err(|e| error!("Radar snapshot loading failed: {e}")).ok()
    }

    /// PNG-encoded 256x256 RGBA radar tile.
    pub fn load_tile(&self, snapshot: u64, z: u32, x: u32, y: u32) -> Option<Vec<u8>> {
        let result: anyhow::Result<Vec<u8>> = self.tokio_rt.block_on(async {
            let bytes = self
                .tile_client
                .get(format!(
                    "{BASE_URL}/radars/{snapshot}/{z}/{x}/{y}?color=0&use_precip_type=1"
                ))
                .send().await?.error_for_status()?.bytes().await?;
            Ok(bytes.to_vec())
        });
        result
            .inspect_err(|e| error!("Radar tile {z}/{x}/{y} loading failed: {e}"))
            .ok()
    }
}
