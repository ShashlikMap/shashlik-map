use crate::tiles::tile_data::TileData;
use crate::tiles::tiles_provider::{MercatorConverter, TilesMessage, TilesProvider, TilesProviderStore};
use futures::{Stream};
use futures::channel::mpsc::{UnboundedSender, unbounded};
use geo::{Area, Convert};
use geo::Winding;
use geo_types::{coord, Coord, LineString, Rect};
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
use std::time::SystemTime;
use osm::map::NatureKind::Water;
use osm::source::reqwest_source::ReqwestSource;
use crate::MAX_ZOOM_LEVEL;
use crate::tiles::mvt::mvt_tile_store::MvtTileStore;
use crate::tiles::shashlik_v1::ShashlikV1TileStore;

pub trait FeatureProcessor: Send + Sync {
    fn process_poi(
        &self,
        geometry_data: &mut Vec<GeometryData>,
        poi: &MapPointInfo,
        zoom_level: i32,
        local_position: &geo::Coord,
        dpi_scale: f32,
    );

    fn process_line(
        &self,
        geometry_data: &mut Vec<GeometryData>,
        line: LineString<f32>,
        interiors: Vec<LineString<f32>>,
        kind: MapGeomObjectKind,
        line_text_map: &mut HashMap<String, i32>,
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

    pub fn set_mvt_type(&mut self, enabled: bool) {
        let new_store: Box<dyn TilesProviderStore> = if enabled {
            // Box::new(MvtTileStore::new())
            Box::new(ShashlikV1TileStore::new())
        } else {
            Box::new(TileStore::new(ReqwestSource::new()))
        };
        self.set_store(new_store)
    }

    fn set_store(&mut self, store: Box<dyn TilesProviderStore>) {
        self.tile_store = Arc::from(store);

        // TODO Refactor
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

        let (tile_position, bbox) = tile_store.tile_position_bbox(tile_key, Self::BBOX_OVERLAP_OFFSET_SCALE);

        let mut geom = tile_store.load(tile_key);

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
                        &mut geometry_data,
                        line.convert(),
                        vec![],
                        obj_type.kind,
                        &mut line_text_map,
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
                            &mut geometry_data,
                            line,
                            interiors,
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
