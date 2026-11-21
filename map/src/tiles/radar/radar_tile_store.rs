use crate::MAX_ZOOM_LEVEL;
use contour::ContourBuilder;
use geo::Scale;
use geo_types::{LineString, Polygon};
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use image::ImageFormat;
use lyon::geom::point;
use lyon::path::Path;
use osm::tiles::TileKey;
use renderer_common::geometry_data::{GeometryData, GeometryType, ShapeData, StyledRangeInfo};
use renderer_common::style_id::StyleId;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest_middleware::ClientWithMiddleware;
use std::env;
use std::time::Duration;
use tokio::runtime::Runtime;

const HTTP_CACHE_ENABLED: bool = true;

pub struct RadarTileStore {
    tokio_rt: Runtime,
    client: ClientWithMiddleware,
}

impl RadarTileStore {
    pub fn new() -> Self {
        let tokio_rt = Runtime::new().unwrap();
        let mut headers = HeaderMap::new();
        // headers.insert(ORIGIN, HeaderValue::from_static("shashlikmap.com"));
        headers.insert(
            "Ocp-Apim-Subscription-Key",
            HeaderValue::from_static("8d36b1564c574de993ceb90bd5417ffa"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        let client_builder =
            if !HTTP_CACHE_ENABLED || cfg!(target_os = "android") || cfg!(target_os = "ios") {
                // fyi, We don't use http cache on mobile device at this moment
                // It requires to pass a files/cache native folder
                reqwest_middleware::ClientBuilder::new(client)
            } else {
                // fyi, keep in mind that the folder might be read-only. Let's ignore it for now.
                let mut cache_dir =
                    env::current_exe().expect("Failed to get current executable path");
                cache_dir.pop();
                cache_dir.push("radar-http-cache");
                reqwest_middleware::ClientBuilder::new(client).with(Cache(HttpCache {
                    mode: CacheMode::Default,
                    manager: CACacheManager::new(cache_dir, false),
                    options: HttpCacheOptions::default(),
                }))
            };

        let client = client_builder.build();

        Self { tokio_rt, client }
    }

    async fn fetch_tile_inner(
        &self,
        x: i32,
        y: i32,
        z: i32,
    ) -> Result<Vec<u8>, reqwest_middleware::Error> {
        let radar_bytes = self
            .client
            .get(format!(
                "https://api.rainbow.ai/tiles/v1/precip/1795235400/0/{z}/{x}/{y}?color=dbz_u8"
            ))
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?
            .to_vec();
        println!("radar size = {:?}", radar_bytes.len());
        // if radar_bytes.len() > 0 {
        //     std::fs::write("radar_output.png", &radar_bytes).expect("Failed to write PNG");
        // }

        Ok(radar_bytes)
    }

    fn geo_polygon_to_lyon_path(poly: &Polygon<f32>) -> Path {
        let mut builder = Path::builder();

        let mut add_ring = |ring: &LineString<f32>| {
            let coords = &ring.0;
            // A valid closed ring has at least 4 coordinates (3 unique + 1 closing)
            if coords.len() < 4 {
                return;
            }

            builder.begin(point(coords[0].x, coords[0].y));
            // Skip the last coordinate since builder.close() closes the loop
            for coord in &coords[1..coords.len() - 1] {
                builder.line_to(point(coord.x, coord.y));
            }
            builder.close();
        };

        // Add outer boundary and any holes
        add_ring(poly.exterior());
        for hole in poly.interiors() {
            add_ring(hole);
        }

        builder.build()
    }

    pub(crate) fn load(&self, tile_key: &TileKey) -> Vec<GeometryData> {
        let mut ret = vec![];
        let zl = tile_key.zoom_level.min(12);
        let zf = (tile_key.zoom_level - 12).max(0);

        // let zl = tile_key.zoom_level.min(12);
        // // if tile_key.zoom_level > 12 {
        // //     return ret;
        // // }
        //

        let bytes = self
            .tokio_rt
            .block_on(self.fetch_tile_inner(tile_key.tile_x >> zf, tile_key.tile_y >> zf, zl))
            .unwrap_or_default();
        if let Ok(image) = image::load_from_memory_with_format(bytes.as_slice(), ImageFormat::Png) {
            let sub_size = 256u32 >> zf;
            let mask = (1i32 << zf) - 1;
            let x0 = ((tile_key.tile_x & mask) as u32) * sub_size;
            let y0 = ((tile_key.tile_y & mask) as u32) * sub_size;

            let sub = image::imageops::crop_imm(&image, x0, y0, sub_size, sub_size);

            let image = sub.to_image();
            let field_raining: Vec<_> = image
                .pixels()
                .map(|pixel| {
                    let [r, _, _, a] = pixel.0;
                    let snowing = r & 128 == 128;
                    if a == 0 || snowing {
                        0f32
                    } else {
                        (r & 127) as f32 - 32.0
                    }
                })
                .collect();
            let field_snowing: Vec<_> = image
                .pixels()
                .map(|pixel| {
                    let [r, _, _, a] = pixel.0;
                    let snowing = r & 128 == 128;
                    if a == 0 || !snowing {
                        0f32
                    } else {
                        (r & 127) as f32 - 32.0
                    }
                })
                .collect();

            let factor = 2.0f32.powi((MAX_ZOOM_LEVEL - zl + 1) as i32);

            let cc = ContourBuilder::new(sub_size as usize, sub_size as usize, true);
            let hh: Vec<_> = cc
                .contours(field_raining.as_slice(), &[15.0, 30.0, 40.0])
                .unwrap_or_default()
                .into_iter()
                .map(|c| {
                    let (poly, th) = c.into_inner();
                    (th, poly.scale_around_point(factor, factor, (0f32, 0f32)).0)
                })
                .collect();
            let hh2: Vec<_> = cc
                .contours(field_snowing.as_slice(), &[10.0, 20.0, 30.0])
                .unwrap_or_default()
                .into_iter()
                .map(|c| {
                    let (poly, th) = c.into_inner();
                    (th, poly.scale_around_point(factor, factor, (0f32, 0f32)).0)
                })
                .collect();

            hh.into_iter().for_each(|(th, vv)| {
                vv.iter().for_each(|vv| {
                    let path = Self::geo_polygon_to_lyon_path(&vv);
                    let style_id = if th == 15.0 {
                        StyleId::new("raining_light")
                    } else if th == 30.0 {
                        StyleId::new("raining_moderate")
                    } else {
                        StyleId::new("raining_heavy")
                    };
                    ret.push(GeometryData::Shape2(ShapeData {
                        path,
                        geometry_type: GeometryType::Polygon,
                        style_id,
                        index_layer_level: 0,
                        styled_range_info: StyledRangeInfo::new(1, true),
                    }));
                });
            });

            hh2.into_iter().for_each(|(th, vv)| {
                vv.iter().for_each(|vv| {
                    let path = Self::geo_polygon_to_lyon_path(&vv);
                    let style_id = if th == 10.0 {
                        StyleId::new("snowing_light")
                    } else if th == 20.0 {
                        StyleId::new("snowing_moderate")
                    } else {
                        StyleId::new("snowing_heavy")
                    };
                    ret.push(GeometryData::Shape2(ShapeData {
                        path,
                        geometry_type: GeometryType::Polygon,
                        style_id,
                        index_layer_level: 0,
                        styled_range_info: StyledRangeInfo::new(1, true),
                    }));
                });
            });
        }
        ret
    }
}
