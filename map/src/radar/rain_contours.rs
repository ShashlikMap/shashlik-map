use crate::tiles::grid_divider::subdivide_grid;
use contour::ContourBuilder;
use geo::{Area, Simplify};
use geo_types::{Coord, LineString, Polygon};
use image::ImageFormat;

/// Rain intensity categories, reflectivity in dBZ. Values are lower bounds (inclusive).
/// ~0.3 mm/h, ~2.7 mm/h, ~11.5 mm/h by Marshall-Palmer.
pub(crate) const RAIN_THRESHOLDS_DBZ: [f32; RAIN_CATEGORIES] = [15.0, 30.0, 40.0];
pub(crate) const RAIN_CATEGORIES: usize = 3;

/// Value used for pixels without radar coverage
const NO_DATA_DBZ: f32 = -100.0;
/// Polygons (and holes) smaller than this, in squared pixels, are just noise
const MIN_AREA_PX: f32 = 1.0;
/// Ramer–Douglas–Peucker tolerance, in pixels
const SIMPLIFY_EPSILON_PX: f32 = 0.3;
const SMOOTH_ITERATIONS: usize = 2;
/// Same as for vector tiles: low zoom tiles are huge and must be split to follow the globe
const GLOBE_SUBDIVISION_MAX_ZOOM: u32 = 4;
const GLOBE_SUBDIVISION_MIN_AREA: f32 = 9999999999.0;
const MAX_SUBDIVISION_LEVEL: u32 = 18;

/// Rain polygons of one tile in tile-local world coordinates,
/// one list per category in `RAIN_THRESHOLDS_DBZ` order.
/// Categories are nested: heavy areas are also covered by moderate and light ones.
#[derive(Default)]
pub(crate) struct RainTileGeometry {
    pub categories: [Vec<Polygon<f32>>; RAIN_CATEGORIES],
}

impl RainTileGeometry {
    pub fn is_empty(&self) -> bool {
        self.categories.iter().all(|polygons| polygons.is_empty())
    }
}

/// Decodes a `color=dbz_u8` tile and builds rain polygons for it.
pub(crate) fn process_tile(png: &[u8], zoom: u32, world_size: f32) -> anyhow::Result<RainTileGeometry> {
    let (size, field) = decode_dbz(png)?;
    Ok(build_geometry(&field, size, zoom, world_size))
}

/// Returns tile size in pixels and dBZ per pixel (row-major).
/// Format: `dbz = (R & 127) - 32`, `R & 128` is the snow flag (ignored), alpha 0 is no coverage.
fn decode_dbz(png: &[u8]) -> anyhow::Result<(usize, Vec<f32>)> {
    let image = image::load_from_memory_with_format(png, ImageFormat::Png)?.to_rgba8();
    let (width, height) = image.dimensions();
    anyhow::ensure!(width == height, "radar tile must be square, got {width}x{height}");
    let field = image
        .pixels()
        .map(|pixel| {
            let [r, _, _, a] = pixel.0;
            if a == 0 { NO_DATA_DBZ } else { (r & 127) as f32 - 32.0 }
        })
        .collect();
    Ok((width as usize, field))
}

fn build_geometry(field: &[f32], size: usize, zoom: u32, world_size: f32) -> RainTileGeometry {
    let mut geometry = RainTileGeometry::default();
    if !field.iter().any(|&dbz| dbz >= RAIN_THRESHOLDS_DBZ[0]) {
        return geometry;
    }

    let contours = match ContourBuilder::new(size, size, true).contours(field, &RAIN_THRESHOLDS_DBZ) {
        Ok(contours) => contours,
        Err(e) => {
            log::error!("Rain contours failed: {e:?}");
            return geometry;
        }
    };

    let size = size as f32;
    let scale = world_size / size;
    for (category, contour) in contours.into_iter().enumerate() {
        let (multi_polygon, _) = contour.into_inner();
        geometry.categories[category] = multi_polygon
            .into_iter()
            .filter_map(|polygon| clean_polygon(polygon, size))
            .map(|polygon| scale_polygon(polygon, scale))
            .flat_map(|polygon| subdivide_for_globe(polygon, zoom))
            .collect();
    }
    geometry
}

/// Snaps to the tile, drops noise, simplifies and smooths. Coordinates are in pixels.
fn clean_polygon(polygon: Polygon<f32>, size: f32) -> Option<Polygon<f32>> {
    let polygon = clamp_polygon(polygon, size);
    if polygon.unsigned_area() < MIN_AREA_PX {
        return None;
    }
    let polygon = polygon.simplify(SIMPLIFY_EPSILON_PX);
    let (exterior, interiors) = polygon.into_inner();
    let exterior = smooth_ring(exterior, size)?;
    let interiors = interiors
        .into_iter()
        .filter(|ring| Polygon::new(ring.clone(), vec![]).unsigned_area() >= MIN_AREA_PX)
        .filter_map(|ring| smooth_ring(ring, size))
        .collect();
    Some(Polygon::new(exterior, interiors))
}

/// Marching squares interpolation may put points slightly outside of the tile, keep them inside
/// so that neighbour tiles meet exactly at the tile border.
fn clamp_polygon(polygon: Polygon<f32>, size: f32) -> Polygon<f32> {
    let clamp = |ring: LineString<f32>| -> LineString<f32> {
        ring.into_iter()
            .map(|c| Coord { x: c.x.clamp(0.0, size), y: c.y.clamp(0.0, size) })
            .collect()
    };
    let (exterior, interiors) = polygon.into_inner();
    Polygon::new(clamp(exterior), interiors.into_iter().map(clamp).collect())
}

/// Chaikin corner cutting, but vertices lying on the tile border are kept in place,
/// otherwise there would be gaps between neighbour tiles.
fn smooth_ring(ring: LineString<f32>, size: f32) -> Option<LineString<f32>> {
    let on_border = |c: &Coord<f32>| c.x <= 0.0 || c.y <= 0.0 || c.x >= size || c.y >= size;

    let mut points: Vec<Coord<f32>> = ring.0;
    // work with an open ring
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    for _ in 0..SMOOTH_ITERATIONS {
        if points.len() < 3 {
            return None;
        }
        let mut smoothed = Vec::with_capacity(points.len() * 2);
        for i in 0..points.len() {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            if on_border(&a) {
                smoothed.push(a);
            } else {
                smoothed.push(a * 0.75 + b * 0.25);
            }
            if !on_border(&b) {
                smoothed.push(a * 0.25 + b * 0.75);
            }
        }
        smoothed.dedup();
        points = smoothed;
    }
    if points.len() < 3 {
        return None;
    }
    let mut ring = LineString::new(points);
    ring.close();
    Some(ring)
}

fn scale_polygon(polygon: Polygon<f32>, scale: f32) -> Polygon<f32> {
    let scale_ring = |ring: LineString<f32>| -> LineString<f32> {
        ring.into_iter().map(|c| c * scale).collect()
    };
    let (exterior, interiors) = polygon.into_inner();
    Polygon::new(scale_ring(exterior), interiors.into_iter().map(scale_ring).collect())
}

fn subdivide_for_globe(polygon: Polygon<f32>, zoom: u32) -> Vec<Polygon<f32>> {
    if zoom >= GLOBE_SUBDIVISION_MAX_ZOOM || polygon.unsigned_area() < GLOBE_SUBDIVISION_MIN_AREA {
        return vec![polygon];
    }
    subdivide_grid(polygon, MAX_SUBDIVISION_LEVEL.saturating_sub(zoom))
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{BoundingRect, Contains};

    const SIZE: usize = 256;

    fn field(f: impl Fn(f32, f32) -> f32) -> Vec<f32> {
        (0..SIZE * SIZE)
            .map(|i| f((i % SIZE) as f32 + 0.5, (i / SIZE) as f32 + 0.5))
            .collect()
    }

    /// pixel coordinates, no globe subdivision
    fn build(field: &[f32]) -> RainTileGeometry {
        build_geometry(field, SIZE, 10, SIZE as f32)
    }

    fn blob(cx: f32, cy: f32, peak: f32, radius: f32) -> impl Fn(f32, f32) -> f32 {
        move |x, y| {
            let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            peak - (peak - 0.0) * d / radius
        }
    }

    #[test]
    fn no_coverage_gives_empty_geometry() {
        assert!(build(&field(|_, _| NO_DATA_DBZ)).is_empty());
        assert!(build(&field(|_, _| 10.0)).is_empty());
    }

    #[test]
    fn full_rain_covers_whole_tile() {
        let geometry = build(&field(|_, _| 35.0));
        let light = &geometry.categories[0];
        assert_eq!(light.len(), 1);
        let rect = light[0].bounding_rect().unwrap();
        assert_eq!((rect.min().x, rect.min().y), (0.0, 0.0), "{rect:?}");
        assert_eq!((rect.max().x, rect.max().y), (SIZE as f32, SIZE as f32), "{rect:?}");
        assert!((light[0].unsigned_area() - (SIZE * SIZE) as f32).abs() < 1.0);
        assert_eq!(geometry.categories[1].len(), 1);
        assert!(geometry.categories[2].is_empty());
    }

    #[test]
    fn blob_gives_nested_closed_rings() {
        let geometry = build(&field(blob(128.0, 128.0, 50.0, 100.0)));
        for category in &geometry.categories {
            assert_eq!(category.len(), 1);
            let ring = category[0].exterior();
            assert!(ring.is_closed());
            let rect = ring.bounding_rect().unwrap();
            assert!(rect.min().x > 0.0 && rect.max().x < SIZE as f32);
        }
        let [light, moderate, heavy] = &geometry.categories;
        assert!(light[0].contains(&moderate[0]));
        assert!(moderate[0].contains(&heavy[0]));
        // smoothed circle should be close to the ideal area: r = 100 * (1 - 15 / 50) = 70
        let ideal = std::f32::consts::PI * 70.0 * 70.0;
        assert!((light[0].unsigned_area() - ideal).abs() / ideal < 0.03, "{}", light[0].unsigned_area());
    }

    #[test]
    fn ring_of_rain_has_hole() {
        let geometry = build(&field(|x, y| {
            let d = ((x - 128.0).powi(2) + (y - 128.0).powi(2)).sqrt();
            if (40.0..90.0).contains(&d) { 20.0 } else { 0.0 }
        }));
        let light = &geometry.categories[0];
        assert_eq!(light.len(), 1);
        assert_eq!(light[0].interiors().len(), 1);
    }

    #[test]
    fn border_vertices_stay_on_border() {
        // rain crossing the left border
        let geometry = build(&field(blob(0.0, 128.0, 50.0, 100.0)));
        let ring = geometry.categories[0][0].exterior();
        let on_left: Vec<_> = ring.coords().filter(|c| c.x == 0.0).collect();
        assert!(on_left.len() >= 2, "rain must be cut exactly by the tile border");
        assert!(ring.coords().all(|c| c.x >= 0.0));
    }

    #[test]
    fn scales_to_world_size() {
        let geometry = build_geometry(&field(|_, _| 20.0), SIZE, 10, 1024.0);
        let rect = geometry.categories[0][0].bounding_rect().unwrap();
        assert_eq!((rect.max().x, rect.max().y), (1024.0, 1024.0));
    }

    /// Real tiles timing, needs network and RAINBOW_API_KEY. Run in release:
    /// `cargo test -p map --release --lib rain_tiles_timing -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn rain_tiles_timing() {
        use crate::radar::radar_store::RadarStore;
        use std::time::{Duration, Instant};

        let store = RadarStore::new(option_env!("RAINBOW_API_KEY").unwrap()).unwrap();
        let snapshot = store.latest_snapshot().unwrap();
        // Japan area: z7 street-level view tiles and a few low zoom tiles
        let mut tiles: Vec<(u32, u32, u32)> = (111..117).flat_map(|x| (48..54).map(move |y| (7, x, y))).collect();
        tiles.extend([(4, 13, 6), (4, 14, 6), (4, 14, 5), (2, 3, 1), (2, 2, 1), (0, 0, 0)]);

        let mut fetch = vec![];
        let mut process = vec![];
        for (z, x, y) in tiles {
            let t0 = Instant::now();
            let png = store.load_tile(snapshot, z, x, y).unwrap();
            fetch.push(t0.elapsed());
            let t1 = Instant::now();
            let world_size = (renderer_common::MAP_SIZE / (1u64 << z) as f64) as f32;
            let geometry = process_tile(&png, z, world_size).unwrap();
            let time = t1.elapsed();
            process.push(time);
            let vertices: usize = geometry.categories.iter().flatten()
                .map(|p| p.exterior().0.len() + p.interiors().iter().map(|r| r.0.len()).sum::<usize>())
                .sum();
            let polygons: Vec<usize> = geometry.categories.iter().map(|c| c.len()).collect();
            println!("{z}/{x}/{y}: png {:>6} B, fetch {:>7.1?}, process {:>7.2?}, polygons {polygons:?}, vertices {vertices}",
                     png.len(), fetch.last().unwrap(), time);
        }
        let stats = |mut v: Vec<Duration>| {
            v.sort();
            let avg = v.iter().sum::<Duration>() / v.len() as u32;
            format!("min {:.2?}, median {:.2?}, avg {avg:.2?}, max {:.2?}", v[0], v[v.len() / 2], v[v.len() - 1])
        };
        println!("fetch:   {}", stats(fetch));
        println!("process: {}", stats(process));
    }
}
