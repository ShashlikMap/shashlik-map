use crate::tiles::tile_data::TileData;
use crate::tiles::tiles_provider::{MercatorConverter, MercatorProvider, TilesMessage, TilesProvider, TilesProviderStore};
use futures::{Stream};
use futures::channel::mpsc::{UnboundedSender, unbounded};
use geo::{Area, BooleanOps, BoundingRect, Convert, Densify, DensifyHaversine, Haversine};
use geo::Winding;
use geo_types::{coord, Coord, LineString, Rect, Polygon};
use log::error;
use osm::map::{MapGeomObject, MapGeomObjectKind, MapGeometry, MapPointInfo};
use osm::tiles::{TileKey, TileStore};
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use renderer_common::geometry_data::{GeometryData};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::spawn;
use std::time::{Instant, SystemTime};
use geo::line_measures::Densifiable;
use googleprojection::Mercator;
use osm::map::NatureKind::Water;
use osm::source::reqwest_source::ReqwestSource;
use renderer_common::TilesType;
use crate::MAX_ZOOM_LEVEL;
use crate::tiles::CustomTileKey;
use crate::tiles::mvt::mvt_tile_store::MvtTileStore;
use crate::tiles::shashlik_v1::ShashlikV1TileStore;

#[derive(Copy, Clone)]
enum Side {
    Low,
    High,
}

pub trait FeatureProcessor: Send + Sync {
    fn process_poi(
        &self,
        id: i64,
        geometry_data: &mut Vec<GeometryData>,
        poi: &MapPointInfo,
        zoom_level: i32,
        local_position: &geo::Coord,
        dpi_scale: f32,
    );

    fn process_line(
        &self,
        id: i64,
        geometry_data: &mut Vec<GeometryData>,
        line: LineString<f32>,
        interiors: Vec<LineString<f32>>,
        kind: MapGeomObjectKind,
        zoom_level: i32,
        dpi_scale: f32,
    );
}

pub struct DefaultTilesProvider<FP: FeatureProcessor> {
    sender: Option<UnboundedSender<TilesMessage>>,
    tile_store: Arc<dyn TilesProviderStore>,
    per_frame_cache: HashSet<TileKey>,
    actual_cache: Arc<RwLock<HashSet<TileKey>>>,
    last_loaded_zoom_level: Arc<AtomicI32>,
    current_zoom_level: Arc<AtomicI32>,
    loading_map: Arc<RwLock<HashMap<i32, i32>>>,
    dpi_scale: f32,
    feature_processor: Arc<FP>,
}

impl<FP: FeatureProcessor + 'static> DefaultTilesProvider<FP> {
    const BBOX_OVERLAP_OFFSET_SCALE: f64 = 1.005;
    pub fn new(tiles_provider_store: Box<dyn TilesProviderStore>, feature_processor: FP, dpi_scale: f32) -> DefaultTilesProvider<FP> {
        Self {
            sender: None,
            tile_store: Arc::from(tiles_provider_store),
            per_frame_cache: HashSet::new(),
            actual_cache: Arc::new(RwLock::new(HashSet::new())),
            last_loaded_zoom_level: Arc::new(AtomicI32::new(1)),
            current_zoom_level: Arc::new(AtomicI32::new(1)),
            loading_map: Arc::new(RwLock::new(HashMap::new())),
            dpi_scale,
            feature_processor: Arc::new(feature_processor),
        }
    }

    pub fn set_tiles_type(&mut self, tiles_type: TilesType) {
        let tiles_provider_store: Box<dyn TilesProviderStore> = match tiles_type {
            TilesType::MapTiler => Box::new(MvtTileStore::new()),
            TilesType::V0 => Box::new(TileStore::new(ReqwestSource::new())),
            TilesType::V1 =>  Box::new(ShashlikV1TileStore::new()),
        };
        self.set_store(tiles_provider_store)
    }

    fn set_store(&mut self, store: Box<dyn TilesProviderStore>) {
        self.tile_store = Arc::from(store);

        // TODO Refactor + cancel ongoing downloads
        self.per_frame_cache.clear();
        self.loading_map.write().unwrap().clear();
        let to_remove = self.actual_cache.read().unwrap().iter().map(|item| item.as_string_key()).collect();
        self.actual_cache.write().unwrap().clear();
        let sender = self.sender.clone().unwrap();
        sender.unbounded_send(TilesMessage::ToRemove(to_remove)).unwrap()
    }

    fn get_tile_key_data(
        tile_store: Arc<dyn TilesProviderStore>,
        feature_processor: Arc<FP>,
        tile_key: &TileKey,
        dpi_scale: f32,
    ) -> TileData {
        let zoom_level = tile_store.convert_zoom(tile_key.zoom_level);

        
        let (tile_position, bbox) = tile_store.tile_position_bbox(&CustomTileKey(tile_key), Self::BBOX_OVERLAP_OFFSET_SCALE);

        let mut geom = tile_store.load(&CustomTileKey(tile_key));

        // A quick workaround for missing water shape tiles since they are not generated if there is no other data
        if geom.is_empty() {
            let fake_water_rectangle = Rect::new(coord! { x: 0.0, y: -bbox.max().y as f32},
                                                 coord! { x: bbox.max().x as f32, y: 0.0 });
            geom.push((MapGeomObject {
                id: -1,
                kind: MapGeomObjectKind::Nature(Water),
            }, MapGeometry::Poly(fake_water_rectangle.to_polygon())))
        }

        let mut geometry_data: Vec<GeometryData> = vec![];
        let mut qq = 0;
        geom.into_iter()
            .for_each(|(obj_type, geometry)| match geometry {
                MapGeometry::Coord(coord) => {
                    let local_position = coord! { x: coord.x as f64, y: coord.y as f64};
                    match &obj_type.kind {
                        MapGeomObjectKind::Poi(poi) => {
                            feature_processor.process_poi(
                                obj_type.id,
                                &mut geometry_data,
                                poi,
                                zoom_level,
                                &local_position,
                                dpi_scale,
                            );
                        }
                        _ => {}
                    }
                }
                MapGeometry::Line(line) => {
                    feature_processor.process_line(
                        obj_type.id,
                        &mut geometry_data,
                        line.convert(),
                        vec![],
                        obj_type.kind,
                        zoom_level,
                        dpi_scale,
                    );
                }
                MapGeometry::Poly(poly) => {
                    let is_building = matches!(obj_type.kind, MapGeomObjectKind::Building(_));
                    let is_water = matches!(obj_type.kind, MapGeomObjectKind::Nature(Water));
                    let is_visible = !cfg!(target_os = "linux")
                        || zoom_level == MAX_ZOOM_LEVEL
                        // reduce amount of buildings for linux
                        || (zoom_level == (MAX_ZOOM_LEVEL - 1) && is_building && poly.unsigned_area() >= 2.0);

                    let is_visible = !is_building || is_visible;

                    if is_visible {
                        // subdivision is required for globe
                        let t1 = Instant::now();
                        let polygons = Self::subdivide_to_grid(zoom_level, poly, (7 - zoom_level) as u32);
                        qq += t1.elapsed().as_micros() as usize;
                        if polygons.is_none() {
                            error!("No polygons after subdivision")
                        }
                        for poly in polygons.unwrap_or_default() {
                            let (mut line, interiors) = poly.into_inner();
                            let interiors = if is_water {
                                interiors
                            } else {
                                vec![]
                            };

                            if is_building {
                                // the winding might not be the same for building lines,
                                // make it as pipelines default
                                line.make_ccw_winding();
                            }

                            feature_processor.process_line(
                                obj_type.id,
                                &mut geometry_data,
                                line,
                                interiors,
                                obj_type.kind.clone(),
                                zoom_level,
                                dpi_scale,
                            );
                        }
                    }
                }
            });

        println!("tile: {:?}, qq ={:?}",tile_key, qq as f64 / 1000.0);
        let tile_data = TileData {
            key: tile_key.as_string_key(),
            position: tile_position,
            zoom_level,
            bbox,
            geometry_data,
        };

        tile_data
    }

    fn subdivide_to_grid(zoom: i32, polygon: Polygon<f32>, grid_size: u32) -> Option<Vec<Polygon<f32>>> {
        if zoom >= 4 {
            return Some(vec![polygon]);
        }

        let mut ttt = vec![];
        subdivide(polygon, 4096.0, grid_size, &mut ttt);
        println!("zoom: {}, grid_size = {}, l ={}", zoom, grid_size, ttt.len());


        // let rect = polygon.bounding_rect()?; // None only if polygon is empty
        // let (min, max) = (rect.min(), rect.max());
        // let cell_w = (max.x - min.x) / grid_size as f32;
        // let cell_h = (max.y - min.y) / grid_size as f32;
        //
        // let mut cells = Vec::new();
        // for row in 0..grid_size {
        //     for col in 0..grid_size {
        //         let x0 = min.x + col as f32 * cell_w;
        //         let y0 = min.y + row as f32 * cell_h;
        //         let cell_rect = Polygon::new(
        //             LineString::from(vec![
        //                 (x0, y0), (x0 + cell_w, y0), (x0 + cell_w, y0 + cell_h), (x0, y0 + cell_h),
        //             ]),
        //             vec![],
        //         );
        //         cells.extend(polygon.intersection(&cell_rect));
        //     }
        // }
        Some(ttt)
    }
}

/// Clip an open ring (no repeated last point) against a single axis-aligned
/// half-plane. axis: 0 = x, 1 = y.
fn clip_ring(
    ring: &[Coord<f32>],
    axis: u8,
    k: f32,
    side: Side,
    out: &mut Vec<Coord<f32>>,
) {
    out.clear();
    if ring.len() < 3 {
        return;
    }

    let c = |p: &Coord<f32>| if axis == 0 { p.x } else { p.y };
    let inside = |p: &Coord<f32>| match side {
        Side::Low => c(p) <= k,
        Side::High => c(p) >= k,
    };

    fn push(out: &mut Vec<Coord<f32>>, p: Coord<f32>) {
        if out.last().map_or(true, |l| l.x != p.x || l.y != p.y) {
            out.push(p);
        }
    }

    let n = ring.len();
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        let (ai, bi) = (inside(&a), inside(&b));

        if ai {
            push(out, a);
        }
        if ai != bi {
            let (ca, cb) = (c(&a), c(&b));
            let t = (k - ca) / (cb - ca); // safe: sides differ, so ca != cb
            let mut p = Coord {
                x: a.x + (b.x - a.x) * t,
                y: a.y + (b.y - a.y) * t,
            };
            // snap the cut axis exactly onto the grid line
            if axis == 0 {
                p.x = k
            } else {
                p.y = k
            }
            push(out, p);
        }
    }

    // drop wrap-around duplicate
    if out.len() > 1 && out[0] == *out.last().unwrap() {
        out.pop();
    }
}

fn open_ring(ls: &LineString<f32>) -> Vec<Coord<f32>> {
    let mut v = ls.0.clone();
    if v.len() > 1 && v[0] == v[v.len() - 1] {
        v.pop();
    }
    v
}

/// Shoelace, accumulated in f64 to survive large tile coordinates.
fn abs_area(ring: &[Coord<f32>]) -> f64 {
    let n = ring.len();
    if n < 3 {
        return 0.0;
    }
    let mut s = 0.0f64;
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        s += a.x as f64 * b.y as f64 - b.x as f64 * a.y as f64;
    }
    (s * 0.5).abs()
}

fn clip_quadrant(
    poly: &Polygon<f32>,
    mid_x: f32,
    x_side: Side,
    mid_y: f32,
    y_side: Side,
    area_eps: f64,
    tmp: &mut Vec<Coord<f32>>,
    acc: &mut Vec<Coord<f32>>,
) -> Option<Polygon<f32>> {
    let ext = open_ring(poly.exterior());
    clip_ring(&ext, 0, mid_x, x_side, tmp);
    clip_ring(tmp, 1, mid_y, y_side, acc);

    let outer_area = abs_area(acc);
    if acc.len() < 3 || outer_area <= area_eps {
        return None;
    }

    let mut exterior = acc.clone();
    exterior.push(exterior[0]); // geo wants closed rings

    let mut holes = Vec::new();
    let mut hole_area = 0.0f64;
    for h in poly.interiors() {
        let ring = open_ring(h);
        clip_ring(&ring, 0, mid_x, x_side, tmp);
        clip_ring(tmp, 1, mid_y, y_side, acc);

        let a = abs_area(acc);
        if acc.len() >= 3 && a > area_eps {
            hole_area += a;
            let mut hole = acc.clone();
            hole.push(hole[0]);
            holes.push(LineString::from(hole));
        }
    }

    // quadrant sits entirely inside a hole -> nothing to draw
    if hole_area >= outer_area - area_eps {
        return None;
    }

    Some(Polygon::new(LineString::from(exterior), holes))
}

/// Split a polygon into the four quadrants defined by the lines
/// x = mid_x and y = mid_y. Empty and degenerate pieces are dropped.
pub fn split_2x2(
    poly: &Polygon<f32>,
    mid_x: f32,
    mid_y: f32,
    area_eps: f64,
) -> Vec<Polygon<f32>> {
    // cheap reject: polygon doesn't straddle either cut
    if let Some(r) = poly.bounding_rect() {
        let spans_x = r.min().x < mid_x && r.max().x > mid_x;
        let spans_y = r.min().y < mid_y && r.max().y > mid_y;
        if !spans_x && !spans_y {
            return vec![poly.clone()];
        }
    } else {
        return Vec::new();
    }

    let mut tmp = Vec::new();
    let mut acc = Vec::new();
    let mut out = Vec::with_capacity(4);

    for (xs, ys) in [
        (Side::Low, Side::Low),
        (Side::High, Side::Low),
        (Side::Low, Side::High),
        (Side::High, Side::High),
    ] {
        if let Some(p) =
            clip_quadrant(poly, mid_x, xs, mid_y, ys, area_eps, &mut tmp, &mut acc)
        {
            out.push(p);
        }
    }
    out
}

// -----------------------------------------------------------------------------
// Recursive driver
// -----------------------------------------------------------------------------

/// Build a 2^depth x 2^depth grid. `cell` is the finest cell size; cuts are
/// anchored to multiples of it so all polygons in a tile split consistently.
pub fn subdivide(
    poly: Polygon<f32>,
    extent: f32,
    depth: u32,
    out: &mut Vec<Polygon<f32>>,
) {
    if depth == 0 {
        out.push(poly);
        return;
    }
    let cell = extent / ((1u32 << (depth - 1)) as f32);

    let step = cell * (1u32 << (depth - 1)) as f32;
    let r = match poly.bounding_rect() {
        Some(r) => r,
        None => return,
    };
    let mid_x = ((r.min().x / step).floor() + 1.0) * step;
    let mid_y = ((r.min().y / step).floor() + 1.0) * step;

    for part in split_2x2(&poly, mid_x, mid_y, 0f64) {
        subdivide(part, cell, depth - 1, out);
    }
}

impl<FP: FeatureProcessor + 'static> MercatorProvider for DefaultTilesProvider<FP> {
    fn mercator(&self) -> Mercator {
        Mercator::default()
    }
}

impl<FP: FeatureProcessor + 'static> MercatorConverter for DefaultTilesProvider<FP> {
    fn lon_lat_to_world(&self, lon_lat: &Coord<f64>, zoom_level: i32) -> Coord<f64> {
        self.tile_store.lon_lat_to_world(lon_lat, zoom_level)
    }

    fn world_to_lon_lat(&self, xy: &Coord<f64>, zoom_level: i32) -> Coord<f64> {
        self.tile_store.world_to_lon_lat(xy, zoom_level)
    }
}

impl<FP: FeatureProcessor + 'static> TilesProvider
    for DefaultTilesProvider<FP>
{
    // TODO Can we get rid of that? And what would be the better way pass converter to a thread?
    fn inner_converter(&self) -> Arc<dyn MercatorConverter> {
        self.tile_store.clone()
    }

    fn load(&mut self, area_poly: geo_types::Polygon<f64>, zoom_level: i32) {
        let mut current_visible_tiles: HashSet<TileKey> = HashSet::new();
        let mut to_load: HashSet<TileKey> = HashSet::new();

        self.tile_store.tile_ranges(area_poly, zoom_level).into_iter().for_each(|tile_key| {
            current_visible_tiles.insert(tile_key);
            if self.per_frame_cache.insert(tile_key) {
                to_load.insert(tile_key);
            }
        });

        let zoom_level = self.tile_store.convert_zoom(zoom_level);
        self.current_zoom_level.store(zoom_level, Ordering::Relaxed);

        if let Ok(mut actual_cache) = self.actual_cache.try_write() {
            let sender = self.sender.clone().unwrap();

            let last_loaded_zoom_level = self.last_loaded_zoom_level.load(Ordering::Relaxed);

            let removed: HashSet<TileKey> = actual_cache
                .extract_if(|key| {
                    (key.zoom_level == zoom_level && !current_visible_tiles.contains(&key))
                        || (key.zoom_level != last_loaded_zoom_level
                            && last_loaded_zoom_level == zoom_level)
                })
                .collect();

            if !removed.is_empty() {
                sender
                    .unbounded_send(TilesMessage::ToRemove(
                        removed.iter().map(|item| item.as_string_key()).collect()
                    ))
                    .unwrap();
            }
        }

        let removed: HashSet<TileKey> = self
            .per_frame_cache
            .extract_if(|key| !current_visible_tiles.contains(&key))
            .collect();

        if !removed.is_empty() || !to_load.is_empty() {
            let ts = SystemTime::now();
            let tile_store = self.tile_store.clone();
            let current_zoom_level = self.current_zoom_level.clone();
            let actual_cache = self.actual_cache.clone();
            let last_loaded_zoom_level = self.last_loaded_zoom_level.clone();
            let loading_map = self.loading_map.clone();
            let sender = self.sender.clone().unwrap();
            let feature_processor = self.feature_processor.clone();
            let dpi_scale = self.dpi_scale;
            spawn(move || {
                let loading_count = *loading_map
                    .write()
                    .unwrap()
                    .entry(zoom_level)
                    .and_modify(|v| *v = *v + 1)
                    .or_insert(1);
                let data: Vec<(TileKey, TileData)> = to_load
                    .par_iter()
                    .filter_map(|key| {
                        if current_zoom_level.load(Ordering::Relaxed) == zoom_level {
                            let tile_data = Self::get_tile_key_data(
                                tile_store.clone(),
                                feature_processor.clone(),
                                key,
                                dpi_scale,
                            );
                            Some((key.clone(), tile_data))
                        } else {
                            None
                        }
                    })
                    .collect();
                if !data.is_empty() && zoom_level == current_zoom_level.load(Ordering::Relaxed) {
                    if loading_count == 1 {
                        last_loaded_zoom_level.store(zoom_level, Ordering::Relaxed);
                    }

                    actual_cache
                        .write()
                        .unwrap()
                        .extend(data.iter().map(|item| item.0.clone()));

                    error!(
                        "Tiles batch is ready: {:?}",
                        SystemTime::now().duration_since(ts)
                    );
                    sender
                        .unbounded_send(TilesMessage::TilesData(
                            data.into_iter().map(|(_, data)| data).collect(),
                        ))
                        .unwrap();
                }

                loading_map
                    .write()
                    .unwrap()
                    .entry(zoom_level)
                    .and_modify(|v| *v = (*v - 1).max(0))
                    .or_insert(0);
            });
        }
    }

    fn tiles(&mut self) -> impl Stream<Item = TilesMessage> + Send + 'static {
        let (sender, receiver) = unbounded();
        self.sender = Some(sender);

        receiver
    }
}
