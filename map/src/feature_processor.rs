use crate::tiles::shashlik_tiles_provider_v0::FeatureProcessor;
use cgmath::{Vector2, Vector3};
use geo_types::{Coord, LineString};
use lyon::geom::point;
use lyon::path::Path;
use osm::map::{
    HighwayKind, LayerKind, LineKind, MapGeomObjectKind, MapPointInfo, MapPointObjectKind,
    NatureKind,
};
use rand::Rng;
use renderer::draw_commands::{GeometryType, PolylineOptions};
use renderer::geometry_data::{ExtrudedPolygonData, GeometryData, ShapeData, SvgData, TextData};
use renderer::styles::style_id::StyleId;
use seahash::hash;
use std::collections::HashMap;

pub struct ShashlikFeatureProcessor {}

impl ShashlikFeatureProcessor {
    const TRAFFIC_LIGHT_SVG: &'static [u8] = include_bytes!("../svg/traffic_light.svg");
    const PARKING_SVG: &'static [u8] = include_bytes!("../svg/parking.svg");
    const TOILETS_SVG: &'static [u8] = include_bytes!("../svg/toilet.svg");
    const TRAIN_STATION_SVG: &'static [u8] = include_bytes!("../svg/train_station.svg");
    pub fn new() -> Self {
        ShashlikFeatureProcessor {}
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
            HighwayKind::Motorway | HighwayKind::Primary => motorway_width * (zoom / 2.0).max(1.0),
            HighwayKind::Trunk => motorway_width * (zoom / 3.0).max(1.0),
            HighwayKind::Tertiary | HighwayKind::Secondary => motorway_width,

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
}

impl FeatureProcessor for ShashlikFeatureProcessor {
    fn process_poi(
        &self,
        geometry_data: &mut Vec<GeometryData>,
        poi: &MapPointInfo,
        local_position: &Coord,
        dpi_scale: f32,
    ) {
        let icon: Option<(&str, &[u8])> = match poi.kind {
            MapPointObjectKind::TrainStation(is_train) => {
                if is_train {
                    Some(("train_station", Self::TRAIN_STATION_SVG))
                } else {
                    Some(("railway_station", Self::TRAIN_STATION_SVG))
                }
            }
            MapPointObjectKind::TrafficLight => Some(("traffic_light", Self::TRAFFIC_LIGHT_SVG)),
            MapPointObjectKind::Toilet => Some(("toilets", Self::TOILETS_SVG)),
            MapPointObjectKind::Parking => Some(("parking", Self::PARKING_SVG)),
            MapPointObjectKind::PopArea(..) => None,
        };
        if let Some(icon) = icon {
            let style_id = match poi.kind {
                MapPointObjectKind::TrainStation(is_train) => {
                    if is_train {
                        StyleId("train_station")
                    } else {
                        StyleId("railway_station")
                    }
                }
                MapPointObjectKind::TrafficLight => StyleId("poi_traffic_light"),
                MapPointObjectKind::Toilet => StyleId("poi_toilet"),
                _ => StyleId("poi"),
            };

            geometry_data.push(GeometryData::Svg(SvgData {
                icon,
                position: Vector3::from((local_position.x, local_position.y, 0.0))
                    .cast()
                    .unwrap(),
                size: 40.0 * dpi_scale,
                style_id,
                with_collision: true,
            }));
        }

        if !poi.text.is_empty() {
            let id =
                hash(format!("{:?}{}{}", poi.text, local_position.x, local_position.y).as_bytes());
            let y_offset = if icon.is_some() { 30.0 } else { 0.0 };
            geometry_data.push(GeometryData::Text(TextData {
                id,
                text: poi.text.to_uppercase(),
                screen_offset: Vector2::new(0.0, y_offset * dpi_scale),
                size: 40.0 * dpi_scale,
                positions: vec![
                    Vector3::from((local_position.x, local_position.y, 0.0))
                        .cast()
                        .unwrap(),
                ],
            }));
        }
    }

    fn process_line(
        &self,
        geometry_data: &mut Vec<GeometryData>,
        line: LineString,
        kind: MapGeomObjectKind,
        line_text_map: &mut HashMap<String, i32>,
        zoom_level: i32,
        dpi_scale: f32,
    ) {
        let line = line.0;
        if line.len() >= 2 {
            let mut path_builder = Path::builder();
            path_builder.begin(point(line[0].x as f32, line[0].y as f32));

            for &p in line[1..].iter() {
                path_builder.line_to(point(p.x as f32, p.y as f32));
            }
            path_builder.end(false);

            if let Some((style_id, layer_level, geometry_type, name)) = match &kind {
                MapGeomObjectKind::Way(info) => match info.line_kind {
                    LineKind::Highway { kind } => {
                        if kind != HighwayKind::Footway {
                            let show_name = zoom_level <= 3;
                            Some((
                                Self::highway_style_id(&kind),
                                info.layer,
                                GeometryType::Polyline(PolylineOptions {
                                    width: Self::highway_width(&kind, zoom_level as f32),
                                    ..Default::default()
                                }),
                                if show_name {
                                    info.name_en.clone()
                                } else {
                                    None
                                },
                            ))
                        } else {
                            None
                        }
                    }
                    LineKind::Railway { .. } => {
                        // TODO Ignore rails tunnels for a while
                        if info.layer_kind != LayerKind::Tunnel {
                            Some((
                                StyleId("rails"),
                                info.layer,
                                GeometryType::Polyline(PolylineOptions {
                                    width: 0.3 * zoom_level.max(1) as f32,
                                    ..Default::default()
                                }),
                                None,
                            ))
                        } else {
                            None
                        }
                    }
                },
                MapGeomObjectKind::AdminLine => Some((
                    StyleId("admin_line"),
                    0,
                    GeometryType::Polyline(PolylineOptions {
                        width: 250.0f32,
                        ..Default::default()
                    }),
                    None,
                )),
                MapGeomObjectKind::Nature(kind) => {
                    let style_id = match kind {
                        NatureKind::Ground => StyleId("ground"),
                        NatureKind::Park => StyleId("park"),
                        NatureKind::Forest => StyleId("forest"),
                        NatureKind::Water => StyleId("water"),
                    };
                    Some((style_id, -100, GeometryType::Polygon, None))
                }
                MapGeomObjectKind::Building(_) => {
                    Some((StyleId("building"), -100, GeometryType::Polygon, None))
                }
                _ => None,
            } {
                if let MapGeomObjectKind::Building(level) = kind
                    && zoom_level == 0
                {
                    let level = if level == 0 {
                        rand::rng().random_range(2..=3)
                    } else {
                        level
                    };
                    geometry_data.push(GeometryData::ExtrudedPolygon(ExtrudedPolygonData {
                        path: path_builder.build(),
                        height: level as f32 / 2.0,
                    }));
                } else {
                    geometry_data.push(GeometryData::Shape(ShapeData {
                        path: path_builder.build(),
                        geometry_type,
                        style_id,
                        index_layer_level: layer_level as i8,
                        is_screen: false,
                    }));
                }

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
                                size: 30.0 * dpi_scale,
                                positions: line
                                    .iter()
                                    .map(|item| Vector3::new(item.x as f32, item.y as f32, 0.0))
                                    .collect(),
                            }));
                        }
                    }
                }
            }
        }
    }
}
