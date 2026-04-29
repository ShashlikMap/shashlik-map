use crate::tiles::tile_data::TileData;
use crate::tiles::tiles_provider::{TilesMessage, TilesProvider};
use futures::Stream;
use futures::channel::mpsc::{UnboundedSender, unbounded};
use geo::{Convert, Intersects};
use geo::Winding;
use geo_types::{coord, LineString, Rect};
use googleprojection::Mercator;
use log::error;
use osm::map::{
    MapGeomObjectKind, MapGeometry, MapPointInfo,
};
use osm::source::TileSource;
use osm::tiles::{TILES_COUNT, TileKey, TileStore, calc_tile_ranges};
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use renderer::geometry_data::{GeometryData};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::spawn;
use std::time::SystemTime;

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
        let tile_rect = tile_key.calc_tile_boundary(1.0);

        let tile_rect_origin = Self::lon_lat_to_world(&tile_rect.min());
        let tile_rect_max = Self::lon_lat_to_world(&tile_rect.max());
        let tile_rect_size = tile_rect_max - tile_rect_origin;

        let geom = tile_store.load_geometries(&tile_key);

        let tile_position = [tile_rect_origin.x, tile_rect_origin.y, 0.0].into();

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
                    let mut line = poly.into_inner().0;
                    if let MapGeomObjectKind::Building(_) = obj_type.kind {
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
            });

        let tile_data = TileData {
            key: tile_key.as_string_key(),
            position: tile_position,
            zoom_level,
            // can be negative
            size: (tile_rect_size.x.abs(), tile_rect_size.y.abs()),
            geometry_data,
        };

        tile_data
    }
}

impl<S: TileSource, FP: FeatureProcessor + 'static> TilesProvider
    for ShashlikTilesProviderV0<S, FP>
{
    fn load(&mut self, area_lonlat: Rect, area_poly: geo_types::Polygon<f64>, zoom_level: i32) {
        let ranges = calc_tile_ranges(TILES_COUNT, zoom_level, &area_lonlat);
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
                let tile_rect = tile_key.calc_tile_boundary(1.0);
                if area_poly.intersects(&tile_rect) {
                    current_visible_tiles.insert(tile_key);
                    if self.per_frame_cache.insert(tile_key) {
                        to_load.insert(tile_key);
                    }
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

    fn world_to_lon_lat(xy: &geo_types::Coord<f64>) -> geo_types::Coord<f64> {
        let xy: (f64, f64) = (*xy).into();
        Mercator::with_size(1)
            .from_pixel_to_ll(&xy, 22)
            .unwrap()
            .into()
    }
}
