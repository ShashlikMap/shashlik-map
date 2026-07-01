use crate::tiles::tile_data::TileData;
use crate::tiles::tiles_provider::{TilesMessage, TilesProvider};
use futures::Stream;
use futures::channel::mpsc::{UnboundedSender, unbounded};
use geo::{Area, Convert, CoordsIter, Intersects, Scale};
use geo::Winding;
use geo_types::{coord, Coord, LineString, Rect};
use googleprojection::Mercator;
use log::error;
use osm::map::{
    MapGeomObjectKind, MapGeometry, MapPointInfo,
};
use osm::source::TileSource;
use osm::tiles::{TILES_COUNT, TileKey, TileStore, calc_tile_ranges, TILE_OVERLAP_PERCENT, TileRanges};
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use renderer::geometry_data::{GeometryData};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::spawn;
use std::time::SystemTime;
use glam::DVec3;
use crate::{read_mvt_tile};

pub trait FeatureProcessor: Send + Sync {
    fn process_poi(
        &self,
        geometry_data: &mut Vec<GeometryData>,
        poi: &MapPointInfo,
        local_position: &geo::Coord,
        dpi_scale: f32,
    );

    fn process_line(
        &self,
        geometry_data: &mut Vec<GeometryData>,
        line: LineString,
        kind: MapGeomObjectKind,
        line_text_map: &mut HashMap<String, i32>,
        zoom_level: i32,
        dpi_scale: f32,
    );
}

pub struct TileMetersBounds {
    pub min_x: f64, // Left edge
    pub min_y: f64, // Bottom edge
    pub max_x: f64, // Right edge
    pub max_y: f64, // Top edge
}

/// Calculates the ground bounding box in Web Mercator Meters for a 512x512 tile.
pub fn tile_id_to_mercator_meters(tx: u32, ty: u32, zoom: u32) -> TileMetersBounds {
    const EXTENT: f64 = 4194304.342789244;
    const MAP_SIZE: f64 = EXTENT * 2.0;

    // Account for 512x512 tile grid scale (zoom - 1)
    // let effective_zoom = if zoom > 0 { zoom - 1 } else { 0 };
    let num_tiles = (1 << zoom) as f64;

    // 1. Find the percentage coordinates (0.0 to 1.0) for the tile edges
    let norm_left = tx as f64 / num_tiles;
    let norm_right = (tx + 1) as f64 / num_tiles;

    let norm_top = ty as f64 / num_tiles;
    let norm_bottom = (ty + 1) as f64 / num_tiles;

    // 2. Convert percentages back to Web Mercator Meters
    let min_x = (norm_left * MAP_SIZE);
    let max_x = (norm_right * MAP_SIZE);

    // Invert Y because Tile Y=0 is the TOP of the world, but Meter Y=positive is NORTH
    let max_y = (norm_bottom * MAP_SIZE);
    let min_y = (norm_top * MAP_SIZE);

    TileMetersBounds { min_x, min_y, max_x, max_y }
}

pub struct ShashlikTilesProviderV0<S: TileSource, FP: FeatureProcessor> {
    sender: Option<UnboundedSender<TilesMessage>>,
    tile_store: Arc<TileStore<S>>,
    per_frame_cache: HashSet<TileKey>,
    actual_cache: Arc<RwLock<HashSet<TileKey>>>,
    last_loaded_zoom_level: Arc<AtomicI32>,
    current_zoom_level: Arc<AtomicI32>,
    loading_map: Arc<RwLock<HashMap<i32, i32>>>,
    dpi_scale: f32,
    feature_processor: Arc<FP>,
}

impl<S: TileSource, FP: FeatureProcessor + 'static> ShashlikTilesProviderV0<S, FP> {
    const BBOX_OVERLAP_OFFSET_SCALE: f64 = 1.005;
    pub fn new(source: S, feature_processor: FP, dpi_scale: f32) -> ShashlikTilesProviderV0<S, FP> {
        Self {
            sender: None,
            tile_store: Arc::new(TileStore::new(source)),
            per_frame_cache: HashSet::new(),
            actual_cache: Arc::new(RwLock::new(HashSet::new())),
            last_loaded_zoom_level: Arc::new(AtomicI32::new(1)),
            current_zoom_level: Arc::new(AtomicI32::new(1)),
            loading_map: Arc::new(RwLock::new(HashMap::new())),
            dpi_scale,
            feature_processor: Arc::new(feature_processor),
        }
    }

    fn get_tile_key_data(
        tile_store: Arc<TileStore<S>>,
        feature_processor: Arc<FP>,
        tile_key: &TileKey,
        dpi_scale: f32,
    ) -> TileData {
        let zoom_level = tile_key.zoom_level;
        // let tile_rect = tile_key.calc_tile_boundary(TILE_OVERLAP_PERCENT);

        // let tile_rect_origin = Self::lon_lat_to_world(&tile_rect.min());
        let bounds = tile_id_to_mercator_meters(tile_key.tile_x as u32, tile_key.tile_y as u32, tile_key.zoom_level as u32);
        let tile_position: DVec3 = DVec3::new(bounds.min_x, bounds.min_y, 0.0);

        // let tile_rect_original = tile_key.calc_tile_boundary(1.00);
        // let tile_rect_original_min = Self::lon_lat_to_world(&tile_rect_original.min());
        // let tile_rect_original_max = Self::lon_lat_to_world(&tile_rect_original.max());

        let bbox = Rect::new(coord! {x: bounds.min_x, y: bounds.min_y},
                             coord! {x: bounds.max_x, y: bounds.max_y}).scale(Self::BBOX_OVERLAP_OFFSET_SCALE);

        // let initial_coord: Coord<f64> = (139.757080078125, 35.68798828125).into();
        // let mut map_tile_key = tile_key.clone();
        // let cc = Self::lon_lat_to_world2(&initial_coord, map_tile_key.tilel_zl());
        // map_tile_key.tile_x = (cc.x / 512.0) as i32;
        // map_tile_key.tile_y = (cc.y / 512.0) as i32;
        let geom = read_mvt_tile(tile_store.load_map_tiler(&tile_key).as_slice()).unwrap_or_default();

        let mut geometry_data: Vec<GeometryData> = vec![];
        let mut line_text_map = HashMap::new();
        geom.into_iter()
            .for_each(|(obj_type, geometry)| match geometry {
                MapGeometry::Coord(coord) => {
                    let local_position = coord! { x: coord.x as f64, y: coord.y as f64};
                    match &obj_type.kind {
                        MapGeomObjectKind::Poi(poi) => {
                            feature_processor.process_poi(
                                &mut geometry_data,
                                poi,
                                &local_position,
                                dpi_scale,
                            );
                        }
                        _ => {}
                    }
                }
                MapGeometry::Line(line) => {
                    feature_processor.process_line(
                        &mut geometry_data,
                        line.convert(),
                        obj_type.kind,
                        &mut line_text_map,
                        zoom_level,
                        dpi_scale,
                    );
                }
                MapGeometry::Poly(poly) => {
                    let is_building = matches!(obj_type.kind, MapGeomObjectKind::Building(_));

                    let is_visible = !cfg!(target_os = "linux")
                        || zoom_level == 0
                        // reduce amount of buildings for linux
                        || (zoom_level == 1 && is_building && poly.unsigned_area() >= 2.0);

                    let is_visible = !is_building || is_visible;

                    if is_visible {
                        let mut line = poly.into_inner().0;

                        if is_building {
                            // the winding might not be the same for building lines,
                            // make it as pipelines default
                            line.make_ccw_winding();
                        }

                        feature_processor.process_line(
                            &mut geometry_data,
                            line.convert(),
                            obj_type.kind,
                            &mut line_text_map,
                            zoom_level,
                            dpi_scale,
                        );
                    }
                }
            });

        let tile_data = TileData {
            key: tile_key.as_string_key(),
            position: tile_position,
            zoom_level,
            bbox,
            geometry_data,
        };

        tile_data
    }
}

fn mercator_meters_to_512_tile(mx: f64, my: f64, zoom: u32) -> (u32, u32) {
    const EXTENT: f64 = 4194304.342789244;
    const MAP_SIZE: f64 = EXTENT * 2.0;

    // Convert meters to a normalized 0.0 to 1.0 range
    let norm_x = (mx) / MAP_SIZE;
    let norm_y = (my) / MAP_SIZE; // Flipped because Tile Y increases downwards

    // At 512x512 scale, the map divides by half as many tiles at the same visual density.
    // We adjust by shifting the effective zoom by -1 for the grid division.
    let effective_zoom = if zoom > 0 { zoom - 1 } else { 0 };
    let num_tiles = (1 << zoom) as f64;

    let tx = (norm_x * num_tiles).floor() as u32;
    let ty = (norm_y * num_tiles).floor() as u32;

    let max_tile = (1 << zoom) - 1;
    (tx.min(max_tile), ty.min(max_tile))
}

pub fn calc_tile_ranges2(zoom_level: i32, area_poly: geo_types::Polygon<f64>) -> TileRanges {
    let mut min_x = u32::MAX;
    let mut max_x = u32::MIN;
    let mut min_y = u32::MAX;
    let mut max_y = u32::MIN;

    for coord in area_poly.coords_iter() {
        let (tx, ty) = mercator_meters_to_512_tile(coord.x, coord.y, zoom_level as u32);

        if tx < min_x { min_x = tx; }
        if tx > max_x { max_x = tx; }
        if ty < min_y { min_y = ty; }
        if ty > max_y { max_y = ty; }
    }

    // Returns (min_tile_x, max_tile_x, min_tile_y, max_tile_y)
    TileRanges {
        min_x: min_x as u32,
        max_x: max_x as u32,
        min_y: min_y as u32,
        max_y: max_y as u32,
    }
}

impl<S: TileSource, FP: FeatureProcessor + 'static> TilesProvider
    for ShashlikTilesProviderV0<S, FP>
{
    fn load(&mut self, area_lonlat: Rect, area_poly: geo_types::Polygon<f64>, zoom_level: i32) {
        let ranges = calc_tile_ranges2(zoom_level, area_poly);
        // println!("ranges = {:?}", ranges);

        let mut current_visible_tiles: HashSet<TileKey> = HashSet::new();
        let mut to_load: HashSet<TileKey> = HashSet::new();

        self.current_zoom_level.store(zoom_level, Ordering::Relaxed);

        for tx in ranges.min_x..=ranges.max_x {
            for ty in ranges.min_y..=ranges.max_y {
                let tile_key = TileKey {
                    tile_x: tx as i32,
                    tile_y: ty as i32,
                    zoom_level,
                };

                // FIXME Maybe move "calc_tile_boundary" to tile generator? since we need to calculate all the time and twice(+ before loading)
                // let tile_rect = tile_key.calc_tile_boundary(1.0);
                // if area_poly.intersects(&tile_rect) {
                //     current_visible_tiles.insert(tile_key);
                //     if self.per_frame_cache.insert(tile_key) {
                //         to_load.insert(tile_key);
                //     }
                // }

                current_visible_tiles.insert(tile_key);
                if self.per_frame_cache.insert(tile_key) {
                    to_load.insert(tile_key);
                }
            }
        }

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
                        removed.iter().map(|item| item.as_string_key()).collect(),
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

    fn lon_lat_to_world(lon_lat: &geo_types::Coord<f64>) -> geo_types::Coord<f64> {
        let lon_lat: (f64, f64) = (*lon_lat).into();
        Mercator::with_size(1)
            .from_ll_to_subpixel(&lon_lat, 22)
            .unwrap()
            .into()
    }

    fn lon_lat_to_world2(lon_lat: &geo_types::Coord<f64>, zl: i32) -> geo_types::Coord<f64> {
        let lon_lat: (f64, f64) = (*lon_lat).into();
        Mercator::with_size(512)
            .from_ll_to_subpixel(&lon_lat, zl as usize)
            .unwrap()
            .into()
    }

    fn world_to_lon_lat(xy: &geo_types::Coord<f64>) -> geo_types::Coord<f64> {
        let xy: (f64, f64) = (*xy).into();
        Mercator::with_size(1)
            .from_pixel_to_ll(&xy, 22)
            .unwrap()
            .into()
    }

    fn world_to_lon_lat2(xy: &Coord<f64>, zl: i32) -> Coord<f64> {
        let xy: (f64, f64) = (*xy).into();
        Mercator::with_size(512)
            .from_pixel_to_ll(&xy, zl as usize)
            .unwrap()
            .into()
    }
}
