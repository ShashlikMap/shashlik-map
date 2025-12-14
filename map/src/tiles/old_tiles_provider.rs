use crate::tiles::tile_data::TileData;
use crate::tiles::tiles_provider::{TilesMessage, TilesProvider};
use cgmath::{Vector2, Vector3};
use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures::Stream;
use geo::Intersects;
use geo::Winding;
use geo_types::Rect;
use googleprojection::{Coord, Mercator};
use lyon::geom::point;
use lyon::path::Path;
use osm::map::{
    HighwayKind, LayerKind, LineKind, MapGeomObjectKind, MapGeometry, MapPointObjectKind,
    NatureKind,
};
use osm::source::TileSource;
use osm::tiles::{calc_tile_ranges, TileKey, TileStore, TILES_COUNT};
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
    sender: Option<UnboundedSender<TilesMessage>>,
    tile_store: Arc<TileStore<S>>,
    per_frame_cache: HashSet<TileKey>,
    actual_cache: Arc<RwLock<HashSet<TileKey>>>,
    last_loaded_zoom_level: Arc<AtomicI32>,
    current_zoom_level: Arc<AtomicI32>,
    loading_map: Arc<RwLock<HashMap<i32, i32>>>,
}

impl<S: TileSource> OldTilesProvider<S> {
    const TRAFFIC_LIGHT_SVG: &'static [u8] = include_bytes!("../../svg/traffic_light.svg");
    const PARKING_SVG: &'static [u8] = include_bytes!("../../svg/parking.svg");
    const TOILETS_SVG: &'static [u8] = include_bytes!("../../svg/toilet.svg");
    const TRAIN_STATION_SVG: &'static [u8] = include_bytes!("../../svg/train_station.svg");

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

    fn highway_width(kind: &HighwayKind, zoom: f32) -> f32 {
        // Relative width for zoom 19, OSM:
        // https://github.com/gravitystorm/openstreetmap-carto/blob/23b1cfa7284ac91bb78390fa4cb7f1c2c6350b92/style/roads.mss#L204
        // TODO Figure out the better way to bound line width to zoom
        let motorway_width = 0.80;

        // shows big road better with high zooms
        let zoom = if zoom > 6.0 { zoom * zoom } else { zoom };
        match kind {
            HighwayKind::Motorway
            | HighwayKind::Primary => motorway_width * (zoom / 2.0).max(1.0),
            | HighwayKind::Trunk => motorway_width * (zoom / 3.0).max(1.0),
            HighwayKind::Tertiary
            | HighwayKind::Secondary => motorway_width,

            HighwayKind::MotorwayLink
            | HighwayKind::PrimaryLink
            | HighwayKind::TrunkLink
            | HighwayKind::SecondaryLink
            | HighwayKind::TertiaryLink => motorway_width / 1.687, // 16

            HighwayKind::Residential => motorway_width / 1.588, // 17
            HighwayKind::Unclassified => motorway_width / 1.588, // 17
            HighwayKind::Footway => motorway_width / 15.0,

            _ => motorway_width / 2.454, // 11
        }
    }

    fn get_tile_key_data(tile_store: Arc<TileStore<S>>, tile_key: &TileKey) -> TileData {
        let zoom_level = tile_key.zoom_level;
        let tile_rect = tile_key.calc_tile_boundary(1.0);

        let tile_rect_origin = Self::lat_lon_to_world(&tile_rect.min());
        let tile_rect_max = Self::lat_lon_to_world(&tile_rect.max());
        let tile_rect_size = tile_rect_max - tile_rect_origin;

        let geom = tile_store.load_geometries(&tile_key);

        let tile_position = [tile_rect_origin.x, tile_rect_origin.y, 0.0].into();

        let mut geometry_data: Vec<GeometryData> = vec![];
        let mut line_text_map = HashMap::new();
        geom.into_iter().for_each(|(obj_type, geometry)| {
            match geometry {
                MapGeometry::Coord(coord) => {
                    let local_position = Self::lat_lon_to_world(&coord) - tile_rect_origin;
                    match &obj_type.kind {
                        MapGeomObjectKind::Poi(poi) => {
                            let icon: Option<(&str, &[u8])> = match &poi.kind {
                                MapPointObjectKind::TrainStation => {
                                    let id = seahash::hash(
                                        format!(
                                            "{:?}{}{}",
                                            poi.text, local_position.x, local_position.y
                                        )
                                        .as_bytes(),
                                    );
                                    geometry_data.push(GeometryData::Text(TextData {
                                        id,
                                        text: poi.text.to_uppercase(),
                                        screen_offset: Vector2::new(0.0, 25.0),
                                        size: 40.0,
                                        positions: vec![Vector3::from((
                                            local_position.x,
                                            local_position.y,
                                            0.0,
                                        )).cast().unwrap()],
                                    }));
                                    Some(("train_station", Self::TRAIN_STATION_SVG))
                                }
                                MapPointObjectKind::TrafficLight => {
                                    Some(("traffic_light", Self::TRAFFIC_LIGHT_SVG))
                                }
                                MapPointObjectKind::Toilet => Some(("toilets", Self::TOILETS_SVG)),
                                MapPointObjectKind::Parking => Some(("parking", Self::PARKING_SVG)),
                                MapPointObjectKind::PopArea(..) => {
                                    geometry_data.push(GeometryData::Text(TextData {
                                        id: hash(poi.text.as_bytes()),
                                        text: poi.text.to_uppercase(),
                                        screen_offset: Vector2::new(0.0, 0.0),
                                        size: 40.0,
                                        positions: vec![Vector3::from((
                                            local_position.x,
                                            local_position.y,
                                            0.0,
                                        )).cast().unwrap()],
                                    }));
                                    None
                                }
                                _ => None,
                            };
                            let style_id = match &poi.kind {
                                MapPointObjectKind::TrafficLight => StyleId("poi_traffic_light"),
                                MapPointObjectKind::Toilet => StyleId("poi_toilet"),
                                _ => StyleId("poi"),
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
                                    size: 40.0,
                                    style_id,
                                    with_collision: true,
                                }));
                            }
                        }
                        _ => {}
                    }
                }
                MapGeometry::Line(line) => {
                    if let Some((style_id, layer_level, width, name)) = match &obj_type.kind {
                        MapGeomObjectKind::Way(info) => match info.line_kind {
                            LineKind::Highway { kind } => {
                                if kind != HighwayKind::Footway {
                                    let show_name =  tile_key.zoom_level <= 3;
                                    Some((
                                        Self::highway_style_id(&kind),
                                        info.layer,
                                        Self::highway_width(&kind, tile_key.zoom_level as f32),
                                        if show_name { info.name_en.clone() } else { None },
                                    ))
                                } else {
                                    None
                                }
                            }
                            LineKind::Railway { .. } => {
                                // TODO Ignore rails tunnels for a while
                                if info.layer_kind != LayerKind::Tunnel {
                                    Some((StyleId("rails"), info.layer, 0.3 * tile_key.zoom_level.max(1) as f32, None))
                                } else {
                                    None
                                }
                            }
                        },
                        MapGeomObjectKind::AdminLine => {
                            Some((StyleId("admin_line"), 0, 250.0, None))
                        }
                        _ => None,
                    } {
                        let line: Vec<(f64, f64)> = line
                            .0
                            .iter()
                            .map(|item| (Self::lat_lon_to_world(&item) - tile_rect_origin).into())
                            .collect();
                        if line.len() >= 2 {
                            let mut path_builder = Path::builder();
                            path_builder.begin(point(line[0].x() as f32, line[0].y() as f32));

                            for &p in line[1..].iter() {
                                path_builder.line_to(point(p.x() as f32, p.y() as f32));
                            }
                            path_builder.end(false);

                            let options = PolylineOptions {
                                width,
                            };

                            geometry_data.push(GeometryData::Shape(ShapeData {
                                path: path_builder.build(),
                                geometry_type: GeometryType::Polyline(options),
                                style_id,
                                index_layer_level: layer_level as i8,
                                is_screen: false,
                            }));

                            if let Some(name) = name {
                                // TODO When text render along the path is ready, it has to be decided how to reduce the repetitive data inside tile
                                //  So far just accept every 30 item. There might be more then 500 lines with the same name!
                                let name_count = line_text_map
                                    .entry(name.clone())
                                    .and_modify(|entry| *entry += 1)
                                    .or_insert(0);
                                if *name_count % 30 == 0 {
                                    // FIXME TextRenderer has a bug for only 2 coords line, let's skip it for now
                                    if line.len() > 2 {
                                        geometry_data.push(GeometryData::Text(TextData {
                                            id: hash(name.as_bytes()),
                                            text: name.to_uppercase(),
                                            screen_offset: Vector2::new(0.0, 0.0),
                                            size: 30.0,
                                            positions:
                                                line.iter()
                                                    .map(|item| {
                                                        Vector3::new(
                                                            item.x() as f32,
                                                            item.y() as f32,
                                                            0.0,
                                                        )
                                                    })
                                                    .collect(),
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }
                MapGeometry::Poly(poly) => {
                    let mut line_string = poly.into_inner().0;
                    if let MapGeomObjectKind::Building(_) = obj_type.kind {
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

                        if let MapGeomObjectKind::Building(level) = obj_type.kind && zoom_level == 0 {
                            let level = if level == 0 {
                                rand::rng().random_range(2..=3)
                            } else {
                                level
                            };
                            geometry_data.push(GeometryData::ExtrudedPolygon(
                                ExtrudedPolygonData {
                                    path: path_builder.build(),
                                    height: level as f32 / 2.0,
                                },
                            ));
                        } else {
                            let style_id = if obj_type.kind
                                == MapGeomObjectKind::Nature(NatureKind::Water)
                            {
                                StyleId("water")
                            } else if let MapGeomObjectKind::Building(_) = obj_type.kind {
                                StyleId("building")
                            } else if obj_type.kind == MapGeomObjectKind::Nature(NatureKind::Ground)
                            {
                                StyleId("ground")
                            } else if obj_type.kind == MapGeomObjectKind::Nature(NatureKind::Park) {
                                StyleId("park")
                            } else {
                                StyleId("forest")
                            };

                            geometry_data.push(GeometryData::Shape(ShapeData {
                                path: path_builder.build(),
                                geometry_type: GeometryType::Polygon,
                                style_id,
                                index_layer_level: -100, //no dedicated layer level for polygon in tiles-gen v1
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
    fn load(&mut self, area_latlon: Rect, area_poly: geo_types::Polygon<f64>, zoom_level: i32) {
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

                    sender
                        .unbounded_send(TilesMessage::TilesData(
                            data.into_iter().map(|(_, data)| data).collect(),
                        ))
                        .unwrap();
                }

                loading_map
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
