use fast_mvt::serde_json;
use log::error;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use std::time::Duration;
use tokio::runtime::Runtime;

const BASE_URL: &str = "https://api.rainbow.ai/tiles/v1";
const AUTH_HEADER: &str = "Ocp-Apim-Subscription-Key";
/// Radar based precipitation observation (transparent outside of radar coverage).
/// fyi, "radars" layer can't return raw dBZ, "precip-global" is a satellite/model based forecast.
const LAYER: &str = "precip";

#[derive(Deserialize)]
struct SnapshotResponse {
    snapshot: u64,
}

/// Thin HTTP client for rainbow.ai precipitation tiles.
/// No HTTP cache here: processed geometry is cached instead.
/// https://doc.rainbow.ai/api-ref/scalar?spec=tiles
pub(crate) struct RadarStore {
    tokio_rt: Runtime,
    client: reqwest::Client,
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

        Ok(Self {
            tokio_rt: Runtime::new()?,
            client,
        })
    }

    /// Latest available snapshot (unix timestamp, seconds).
    pub fn latest_snapshot(&self) -> Option<u64> {
        let result: anyhow::Result<u64> = self.tokio_rt.block_on(async {
            let bytes = self
                .client
                .get(format!("{BASE_URL}/snapshot?layer={LAYER}"))
                .send().await?.error_for_status()?.bytes().await?;
            Ok(serde_json::from_slice::<SnapshotResponse>(&bytes)?.snapshot)
        });
        result.inspect_err(|e| error!("Rain snapshot loading failed: {e}")).ok()
    }

    /// PNG-encoded 256x256 tile with raw reflectivity, see `rain_contours::decode_dbz`.
    /// Forecast offset 0 is the current observation.
    pub fn load_tile(&self, snapshot: u64, z: u32, x: u32, y: u32) -> Option<Vec<u8>> {
        let result: anyhow::Result<Vec<u8>> = self.tokio_rt.block_on(async {
            let bytes = self
                .client
                .get(format!("{BASE_URL}/{LAYER}/{snapshot}/0/{z}/{x}/{y}?color=dbz_u8"))
                .send().await?.error_for_status()?.bytes().await?;
            Ok(bytes.to_vec())
        });
        result
            .inspect_err(|e| error!("Rain tile {z}/{x}/{y} loading failed: {e}"))
            .ok()
    }
}
