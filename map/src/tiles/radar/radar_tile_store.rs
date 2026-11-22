use std::fs;
use crate::MAX_ZOOM_LEVEL;
use crate::tiles::reqwest_client::TilesReqwestClientBuilder;
use contour::{ContourBuilder, Float};
use geo::{Scale, Translate};
use geo_types::{LineString, Polygon};
use image::ImageFormat;
use log::error;
use lyon::geom::point;
use lyon::path::Path;
use osm::tiles::TileKey;
use renderer_common::geometry_data::{GeometryData, GeometryType, ShapeData, StyledRangeInfo};
use renderer_common::style_id::StyleId;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest_middleware::ClientWithMiddleware;
use serde::Deserialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::runtime::Runtime;
use crate::tiles::tiles_provider::TilesProviderStore;

pub struct RadarTileStore {
    tokio_rt: Runtime,
    client: ClientWithMiddleware,
    api_key_set: bool,
    snapshot: Arc<AtomicU64>,
}

#[derive(Deserialize)]
struct SnapshotResponse {
    snapshot: u64,
}

impl RadarTileStore {
    const MAX_RADAR_ZOOM_LEVEL: i32 = 12;
    const RAINING_THRESHOLDS: [f32; 3] = [15.0, 30.0, 40.0];
    const RAINING_STYLES: [&str; 3] = ["raining_light", "raining_moderate", "raining_heavy"];

    const SNOWING_THRESHOLDS: [f32; 3] = [10.0, 20.0, 30.0];
    const SNOWING_STYLES: [&str; 3] = ["snowing_light", "snowing_moderate", "snowing_heavy"];

    pub fn new() -> Self {
        let tokio_rt = Runtime::new().unwrap();
        let mut headers = HeaderMap::new();
        let api_key = option_env!("RAINBOW_API_KEY");
        if let Some(api_key) = api_key {
            headers.insert(
                "Ocp-Apim-Subscription-Key",
                HeaderValue::from_static(api_key),
            );
        }
        let client = TilesReqwestClientBuilder::create(Some("radar-http-cache"), headers);

        Self {
            tokio_rt,
            client,
            api_key_set: api_key.is_some(),
            snapshot: Arc::new(0.into()),
        }
    }

    async fn get_latest_snapshot(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let current_snapshot = self.snapshot.load(Ordering::Relaxed);
        if current_snapshot > 0 {
            return Ok(current_snapshot);
        }
        let res = self
            .client
            .get("https://api.rainbow.ai/tiles/v1/snapshot?layer=precip")
            .send()
            .await?
            .error_for_status()?
            .json::<SnapshotResponse>()
            .await?;
        self.snapshot.store(res.snapshot, Ordering::Relaxed);
        println!("New radar snapshot: {:?}", res.snapshot);
        Ok(res.snapshot)
    }

    async fn fetch_tile_inner(&self, x: i32, y: i32, z: i32) -> Result<Vec<u8>, anyhow::Error> {
        if !self.api_key_set {
            let fake_radar_bytes = fs::read("./rainbow_dbz_u8_circles_32x32_small.png")?;
            println!("KIOL fake_radar_bytes = {:?}", fake_radar_bytes.len());
            return Ok(fake_radar_bytes)
        }
        if let Ok(snapshot) = self.get_latest_snapshot().await
            && snapshot > 0
        {
            let radar_bytes = self
                .client
                .get(format!(
                    "https://api.rainbow.ai/tiles/v1/precip/{snapshot}/0/{z}/{x}/{y}?color=dbz_u8"
                ))
                .send()
                .await?
                .error_for_status()?
                .bytes()
                .await?
                .to_vec();

            Ok(radar_bytes)
        } else {
            error!("Can't get radar snapshot");
            Ok(vec![])
        }
    }

    fn geo_polygon_to_lyon_path(poly: &Polygon<f32>) -> Path {
        let mut builder = Path::builder();

        let mut add_ring = |ring: &LineString<f32>| {
            let coords = &ring.0;
            if coords.len() < 4 {
                return;
            }

            builder.begin(point(coords[0].x, coords[0].y));
            for coord in &coords[1..coords.len() - 1] {
                builder.line_to(point(coord.x, coord.y));
            }
            builder.close();
        };

        add_ring(poly.exterior());
        for hole in poly.interiors() {
            add_ring(hole);
        }

        builder.build()
    }

    pub(crate) fn load(&self, tile_store: Arc<dyn TilesProviderStore>, tile_key: &TileKey) -> Vec<GeometryData> {
        let mut geometry_data = vec![];
        let zoom = tile_store.convert_zoom(tile_key.zoom_level);
        let z_limited = zoom.min(Self::MAX_RADAR_ZOOM_LEVEL);
        let z_diff = (zoom - Self::MAX_RADAR_ZOOM_LEVEL).max(0);

        let bytes = self
            .tokio_rt
            .block_on(self.fetch_tile_inner(
                tile_key.tile_x >> z_diff,
                tile_key.tile_y >> z_diff,
                z_limited,
            ))
            .unwrap_or_default();
        if let Ok(image) = image::load_from_memory_with_format(bytes.as_slice(), ImageFormat::Png) {
            let sub_size = 256u32 >> z_diff;
            let mask = (1i32 << z_diff) - 1;
            let x0 = ((tile_key.tile_x & mask) as u32) * sub_size;
            let y0 = ((tile_key.tile_y & mask) as u32) * sub_size;

            let sub = image::imageops::crop_imm(&image, x0, y0, sub_size, sub_size);

            let image = sub.to_image();

            let (field_raining, field_snowing): (Vec<_>, Vec<_>) = image
                .pixels()
                .map(|pixel| {
                    let [r, _, _, a] = pixel.0;
                    if a == 0 {
                        return (0.0, 0.0);
                    }
                    let snowing = r & 128 == 128;
                    let v = (r & 127) as f32 - 32.0;
                    if snowing { (0f32, v) } else { (v, 0.32) }
                })
                .unzip();

            let scale_factor = 2.0f32.powi(MAX_ZOOM_LEVEL - z_limited + 1);

            let contour_builder = ContourBuilder::new(sub_size as usize, sub_size as usize, true);
            let raining_contours = Self::create_contour(
                &contour_builder,
                field_raining,
                scale_factor,
                !self.api_key_set,
                &Self::RAINING_THRESHOLDS,
                &Self::RAINING_STYLES,
            );
            let snowing_contours = Self::create_contour(
                &contour_builder,
                field_snowing,
                scale_factor,
                !self.api_key_set,
                &Self::SNOWING_THRESHOLDS,
                &Self::SNOWING_STYLES,
            );
            Self::fill_geometry_data(&mut geometry_data, raining_contours);
            Self::fill_geometry_data(&mut geometry_data, snowing_contours);
        }
        geometry_data
    }

    fn fill_geometry_data(
        data: &mut Vec<GeometryData>,
        contours: Vec<(String, Vec<Polygon<Float>>)>,
    ) {
        contours.into_iter().for_each(|(style, polygons)| {
            let style_id = StyleId::new(style);
            polygons.iter().for_each(|vv| {
                let path = Self::geo_polygon_to_lyon_path(&vv);
                data.push(GeometryData::Shape(ShapeData {
                    path,
                    geometry_type: GeometryType::Polygon,
                    style_id: style_id.clone(),
                    index_layer_level: 0,
                    styled_range_info: StyledRangeInfo::new(1, true),
                    feature_layer_tag: Some("radar_tiles".to_string()),
                }));
            });
        });
    }

    fn create_contour(
        contour_builder: &ContourBuilder,
        field: Vec<f32>,
        scale_factor: f32,
        is_fake: bool,
        thresholds: &[Float],
        styles: &[&'static str],
    ) -> Vec<(String, Vec<Polygon<Float>>)> {
        contour_builder
            .contours(field.as_slice(), thresholds)
            .unwrap_or_default()
            .into_iter()
            .map(|c| {
                let (poly, th) = c.into_inner();
                let index = thresholds.iter().position(|p| *p == th).unwrap();
                let style = styles[index];

                (
                    style.to_string(),
                    // FIXME the only way to present it on screenshot tests due to V0 tiles are used there
                    // Need to use fake MapTiler tiles for screenshot tests
                    if is_fake {
                        poly.scale(scale_factor).translate(0f32, -500f32)
                    } else {
                        poly.scale_around_point(scale_factor, scale_factor, (0f32, 0f32))
                    }.0,
                )
            })
            .collect()
    }
}
