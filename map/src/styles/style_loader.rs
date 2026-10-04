use crate::styles::FeatureStyle;
use error_stack::{Report, ResultExt};
use fast_mvt::serde_json;
use log::error;
use std::fs;
use std::time::Duration;
use thiserror::Error;

pub struct StyleLoader;

#[derive(Debug, Error)]
enum StylesFetchError {
    #[error("Internal")]
    Internal,
}

impl StyleLoader {
    pub fn load(local_file: bool) -> Vec<FeatureStyle> {
        if local_file {
            return serde_json::from_slice(fs::read("styles_v0.json").as_ref().unwrap()).unwrap();
        }

        let styles = Self::styles();
        if let Err(err) = styles.as_ref() {
            error!("Error loading styles: {:?}", err);
        }
        styles.unwrap_or_default()
    }

    fn styles() -> Result<Vec<FeatureStyle>, Report<StylesFetchError>> {
        let client = reqwest::blocking::Client::builder()
            .tcp_keepalive(Duration::from_secs(30))
            .build()
            .unwrap();
        let response = client
            .get(
                "http://ec2-3-107-91-243.ap-southeast-2.compute.amazonaws.com:3000/styles_v0.json"
                    .to_string(),
            )
            .send();
        response
            .change_context(StylesFetchError::Internal)
            .and_then(|response| {
                serde_json::from_slice(
                    response
                        .bytes()
                        .change_context(StylesFetchError::Internal)?
                        .as_ref(),
                )
                .change_context(StylesFetchError::Internal)
            })
    }
}
