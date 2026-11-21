use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest::header::HeaderMap;
use reqwest_middleware::ClientWithMiddleware;
use std::env;
use std::path::Path;
use std::time::Duration;

pub(crate) struct TilesReqwestClientBuilder;

impl TilesReqwestClientBuilder {
    pub fn create<P: AsRef<Path>>(disk_cache: Option<P>, headers: HeaderMap) -> ClientWithMiddleware {
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        let client_builder = if let Some(disk_cache) = disk_cache
            && (!cfg!(target_os = "android") && !cfg!(target_os = "ios"))
        {
            // fyi, keep in mind that the folder might be read-only. Let's ignore it for now.
            let mut cache_dir = env::current_exe().expect("Failed to get current executable path");
            cache_dir.pop();
            cache_dir.push(disk_cache);
            reqwest_middleware::ClientBuilder::new(client).with(Cache(HttpCache {
                mode: CacheMode::Default,
                manager: CACacheManager::new(cache_dir, false),
                options: HttpCacheOptions::default(),
            }))
        } else {
            // fyi, We don't use http cache on mobile device at this moment
            // It requires to pass a files/cache native folder
            reqwest_middleware::ClientBuilder::new(client)
        };

        client_builder.build()
    }
}
