use crate::tiles::tile_data::TileData;
use crate::tiles::tiles_provider::TilesProvider;
use cgmath::Vector3;
use futures::Stream;
use futures::channel::mpsc::{UnboundedSender, unbounded};
use geo::Winding;
use geo_types::Rect;
use googleprojection::{Coord, Mercator};
use lyon::geom::point;
use lyon::path::Path;
use old_tiles_gen::map::{
    HighwayKind, LineKind, MapGeomObjectKind, MapGeometry, MapPointObjectKind, NatureKind,
};
use old_tiles_gen::source::TileSource;
use old_tiles_gen::tiles::{TILES_COUNT, TileKey, TileStore, calc_tile_ranges};
use rand::Rng;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use renderer::draw_commands::{GeometryType, PolylineOptions};
use renderer::geometry_data::{ExtrudedPolygonData, GeometryData, ShapeData, SvgData, TextData};
use renderer::styles::style_id::StyleId;
use seahash::hash;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::spawn;

pub struct OldTilesProvider<S: TileSource> {
    sender: Option<UnboundedSender<(Option<TileData>, HashSet<String>)>>,
    tile_store: Arc<TileStore<S>>,
    per_frame_cache: HashSet<TileKey>,
    actual_cache: Arc<RwLock<HashSet<TileKey>>>,
    last_loaded_zoom_level: Arc<AtomicI32>,
    current_zoom_level: Arc<AtomicI32>,
    loading_map: Arc<RwLock<HashMap<i32, i32>>>,
}

impl<S: TileSource> OldTilesProvider<S> {
    const TRAFFIC_LIGHT_SVG: &'static [u8] = include_bytes!("../../svg/traffic_light.svg");
    #[allow(dead_code)]
    const PARKING_SVG: &'static [u8] = include_bytes!("../../svg/parking.svg");
    const TOILETS_SVG: &'static [u8] = include_bytes!("../../svg/toilet.svg");

    pub fn new(source: S) -> OldTilesProvider<S> {
        Self {
            sender: None,
            tile_store: Arc::new(TileStore::new(source)),
            per_frame_cache: HashSet::new(),
            actual_cache: Arc::new(RwLock::new(HashSet::new())),
            last_loaded_zoom_level: Arc::new(AtomicI32::new(1)),
            current_zoom_level: Arc::new(AtomicI32::new(1)),
            loading_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn highway_style_id(kind: &HighwayKind) -> StyleId {
        match kind {
            HighwayKind::Motorway | HighwayKind::MotorwayLink => StyleId("highway_motorway"),
            HighwayKind::Primary | HighwayKind::PrimaryLink => StyleId("highway_primary"),
            HighwayKind::Trunk | HighwayKind::TrunkLink => StyleId("highway_trunk"),
            HighwayKind::Secondary | HighwayKind::SecondaryLink => StyleId("highway_secondary"),
            HighwayKind::Tertiary => StyleId("highway_tertiary"),
            HighwayKind::Footway => StyleId("highway_footway"),
            _ => StyleId("highway_default"),
        }
    }

    fn get_tile_key_data(tile_store: Arc<TileStore<S>>, tile_key: &TileKey) -> TileData {
        let tile_rect = tile_key.calc_tile_boundary(1.0);

        let tile_rect_origin = Self::lat_lon_to_world(&tile_rect.min());
        let tile_rect_max = Self::lat_lon_to_world(&tile_rect.max());
        let tile_rect_size = tile_rect_max - tile_rect_origin;

        let geom = tile_store.load_geometries(&tile_key);

        let tile_position = [tile_rect_origin.x, tile_rect_origin.y, 0.0].into();

        let mut rng = rand::rng();
        let mut geometry_data: Vec<GeometryData> = vec![];
        geom.iter().for_each(|(obj_type, geometry)| {
            match geometry {
                MapGeometry::Coord(coord) => {
                    let local_position = Self::lat_lon_to_world(&coord) - tile_rect_origin;
                    match &obj_type.kind {
                        MapGeomObjectKind::Poi(poi) => {
                            let icon: Option<(&str, &[u8])> = match &poi.kind {
                                MapPointObjectKind::TrafficLight => {
                                    Some(("traffic_light", Self::TRAFFIC_LIGHT_SVG))
                                }
                                MapPointObjectKind::Toilet => Some(("toilets", Self::TOILETS_SVG)),
                                MapPointObjectKind::Parking => {
                                    let id = seahash::hash(
                                        format!("PARKING{}{}", local_position.x, local_position.y)
                                            .as_bytes(),
                                    );
                                    geometry_data.push(GeometryData::Text(TextData {
                                        id,
                                        text: "PARKINGPARKINGPARKINGPARKING".to_string(),
                                        position: Vector3::from((
                                            local_position.x,
                                            local_position.y,
                                            0.0,
                                        ))
                                        .cast()
                                        .unwrap(),
                                    }));
                                    // Text instead of icon
                                    // Some(("parking", Self::PARKING_SVG))
                                    None
                                }
                                MapPointObjectKind::PopArea(..) => {
                                    geometry_data.push(GeometryData::Text(TextData {
                                        id: hash(poi.text.as_bytes()),
                                        text: poi.text.clone(),
                                        position: Vector3::from((
                                            local_position.x,
                                            local_position.y,
                                            0.0,
                                        ))
                                        .cast()
                                        .unwrap(),
                                    }));
                                    None
                                }
                                _ => None,
                            };
                            if let Some(icon) = icon {
                                geometry_data.push(GeometryData::Svg(SvgData {
                                    icon,
                                    position: Vector3::from((
                                        local_position.x,
                                        local_position.y,
                                        0.0,
                                    ))
                                    .cast()
                                    .unwrap(),
                                    size: 2.0,
                                    style_id: StyleId("poi"),
                                }));
                            }
                        }
                        _ => {}
                    }
                }
                MapGeometry::Line(line) => {
                    if let Some((style_id, layer_level, width)) = match &obj_type.kind {
                        MapGeomObjectKind::Way(info) => match info.line_kind {
                            LineKind::Highway { kind } => {
                                if kind != HighwayKind::Footway {
                                    Some((Self::highway_style_id(&kind), info.layer, 0.7))
                                } else {
                                    None
                                }
                            }
                            LineKind::Railway { .. } => None,
                        },
                        MapGeomObjectKind::AdminLine => Some((StyleId("admin_line"), 0, 250.0)),
                        _ => None,
                    } {
                        let line: Vec<(f64, f64)> = line
                            .0
                            .iter()
                            .map(|item| (Self::lat_lon_to_world(&item) - tile_rect_origin).into())
                            .collect();
                        if line.len() >= 2 {
                            // println!("new line");
                            let mut path_builder = Path::builder();
                            path_builder.begin(point(line[0].x() as f32, line[0].y() as f32));

                            for &p in line[1..].iter() {
                                path_builder.line_to(point(p.x() as f32, p.y() as f32));
                            }
                            path_builder.end(false);

                            let options = PolylineOptions {
                                width: width as f32,
                            };

                            geometry_data.push(GeometryData::Shape(ShapeData {
                                path: path_builder.build(),
                                geometry_type: GeometryType::Polyline(options),
                                style_id,
                                layer_level: layer_level as i8,
                                is_screen: false,
                            }));
                        }
                    }
                }
                MapGeometry::Poly(poly) => {
                    let mut line_string = poly.exterior().clone();
                    if obj_type.kind == MapGeomObjectKind::Building {
                        line_string.make_cw_winding();
                    }
                    let line: Vec<(f64, f64)> = line_string
                        .0
                        .iter()
                        .map(|item| (Self::lat_lon_to_world(item) - tile_rect_origin).into())
                        .collect();
                    if line.len() >= 2 {
                        let mut path_builder = Path::builder();
                        path_builder.begin(point(line[0].x() as f32, line[0].y() as f32));

                        for &p in line[1..].iter() {
                            path_builder.line_to(point(p.x() as f32, p.y() as f32));
                        }
                        path_builder.end(true);

                        if obj_type.kind == MapGeomObjectKind::Building {
                            let random_height: f32 = rng.random_range(1.0..=10.0);
                            geometry_data.push(GeometryData::ExtrudedPolygon(
                                ExtrudedPolygonData {
                                    path: path_builder.build(),
                                    height: random_height,
                                },
                            ));
                        } else {
                            let style_id = if obj_type.kind
                                == MapGeomObjectKind::Nature(NatureKind::Water)
                            {
                                StyleId("water")
                            } else if obj_type.kind == MapGeomObjectKind::Building {
                                StyleId("building")
                            } else if obj_type.kind == MapGeomObjectKind::Nature(NatureKind::Ground)
                            {
                                StyleId("ground")
                            } else {
                                StyleId("land")
                            };

                            geometry_data.push(GeometryData::Shape(ShapeData {
                                path: path_builder.build(),
                                geometry_type: GeometryType::Polygon,
                                style_id,
                                layer_level: -100, //no dedicated layer level for polygon in tiles-gen v1
                                is_screen: false,
                            }));
                        }
                    }
                }
            }
        });

        let tile_data = TileData {
            key: tile_key.as_string_key(),
            position: tile_position,
            // can be negative
            size: (tile_rect_size.x.abs(), tile_rect_size.y.abs()),
            geometry_data,
        };

        tile_data
    }
}
impl<S: TileSource> TilesProvider for OldTilesProvider<S> {
    fn load(&mut self, area_latlon: Rect, zoom_level: i32) {
        let ranges = calc_tile_ranges(TILES_COUNT, zoom_level, &area_latlon);
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
                        || (key.zoom_level != last_loaded_zoom_level && last_loaded_zoom_level == zoom_level)
                })
                .collect();

            if !removed.is_empty() {
                sender
                    .unbounded_send((
                        None,
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
            let tile_store = self.tile_store.clone();
            let current_zoom_level = self.current_zoom_level.clone();
            let actual_cache = self.actual_cache.clone();
            let last_loaded_zoom_level = self.last_loaded_zoom_level.clone();
            let loading_map = self.loading_map.clone();
            let sender = self.sender.clone().unwrap();
            spawn(move || {
                let mut loading_map = loading_map.write().unwrap();
                let loading_count = loading_map
                    .entry(zoom_level)
                    .and_modify(|v| *v = *v + 1)
                    .or_insert(1);
                let data: Vec<(TileKey, TileData)> = to_load
                    .par_iter()
                    .filter_map(|key| {
                        if current_zoom_level.load(Ordering::Relaxed) == zoom_level {
                            let tile_data = Self::get_tile_key_data(tile_store.clone(), key);
                            Some((key.clone(), tile_data))
                        } else {
                            None
                        }
                    })
                    .collect();
                if !data.is_empty() && zoom_level == current_zoom_level.load(Ordering::Relaxed) {
                    if *loading_count == 1 {
                        last_loaded_zoom_level.store(zoom_level, Ordering::Relaxed);
                    }

                    actual_cache
                        .write()
                        .unwrap()
                        .extend(data.iter().map(|item| item.0.clone()));

                    data.into_iter().for_each(|(_, data)| {
                        sender.unbounded_send((Some(data), HashSet::new())).unwrap();
                    });
                }

                loading_map
                    .entry(zoom_level)
                    .and_modify(|v| *v = (*v - 1).max(0))
                    .or_insert(0);
            });
        }
    }

    fn tiles(
        &mut self,
    ) -> impl Stream<Item = (Option<TileData>, HashSet<String>)> + Send + 'static {
        let (sender, receiver) = unbounded();
        self.sender = Some(sender);

        receiver
    }

    fn lat_lon_to_world(lat_lon: &geo_types::Coord<f64>) -> geo_types::Coord<f64> {
        let lat_lon: (f64, f64) = (*lat_lon).into();
        Mercator::with_size(1)
            .from_ll_to_subpixel(&lat_lon, 22)
            .unwrap()
            .into()
    }

    fn world_to_lat_lon(lat_lon: &geo_types::Coord<f64>) -> geo_types::Coord<f64> {
        let lat_lon: (f64, f64) = (*lat_lon).into();
        Mercator::with_size(1)
            .from_pixel_to_ll(&lat_lon, 22)
            .unwrap()
            .into()
    }
}
