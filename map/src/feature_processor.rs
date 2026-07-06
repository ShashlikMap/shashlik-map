use crate::tiles::default_tiles_provider::FeatureProcessor;
use geo_types::{Coord, LineString};
use glam::{DVec3, Vec2};
use lyon::geom::point;
use lyon::path::Path;
use osm::map::{
    HighwayKind, LayerKind, LineKind, MapGeomObjectKind, MapPointInfo, MapPointObjectKind,
    NatureKind,
};
use rand::RngExt;
use renderer::draw_commands::{GeometryType, PolylineOptions};
use renderer::geometry_data::{ExtrudedPolygonData, GeometryData, LineData, ShapeData, SvgBackground, SvgData, TextData};
use renderer::mesh::mesh::StyledRangeInfo;
use renderer::styles::style_id::StyleId;
use seahash::hash;
use std::collections::HashMap;
use capitalize::Capitalize;
use lyon::lyon_tessellation::{LineCap, LineJoin};
use crate::MAX_ZOOM_LEVEL;

pub struct ShashlikFeatureProcessor {}

impl ShashlikFeatureProcessor {
    const TRAFFIC_LIGHT_SVG: &'static [u8] = include_bytes!("../svg/traffic_light.svg");
    const PARKING_SVG: &'static [u8] = include_bytes!("../svg/parking.svg");
    const TOILETS_SVG: &'static [u8] = include_bytes!("../svg/toilet.svg");
    const TRAIN_STATION_SVG: &'static [u8] = include_bytes!("../svg/train_station.svg");
    const EV_STATION_SVG: &'static [u8] = include_bytes!("../svg/ev_station.svg");
    pub fn new() -> Self {
        ShashlikFeatureProcessor {}
    }

    fn highway_style_id(kind: &HighwayKind) -> StyleId {
        match kind {
            HighwayKind::Motorway | HighwayKind::MotorwayLink => StyleId::new("highway_motorway"),
            HighwayKind::Primary | HighwayKind::PrimaryLink => StyleId::new("highway_primary"),
            HighwayKind::Trunk | HighwayKind::TrunkLink => StyleId::new("highway_trunk"),
            HighwayKind::Secondary | HighwayKind::SecondaryLink => StyleId::new("highway_secondary"),
            HighwayKind::Tertiary => StyleId::new("highway_tertiary"),
            HighwayKind::Footway => StyleId::new("highway_footway"),
            _ => StyleId::new("highway_default"),
        }
    }

    fn highway_width(kind: &HighwayKind, zoom: f32) -> f32 {
        // Relative width for zoom 19, OSM:
        // https://github.com/gravitystorm/openstreetmap-carto/blob/23b1cfa7284ac91bb78390fa4cb7f1c2c6350b92/style/roads.mss#L204
        // TODO Figure out the better way to bound line width to zoom
        let motorway_width = 0.85 * 4.0;

        // shows big road better with high zooms
        let zoom = if zoom >= 6.0 { zoom * zoom } else { zoom * zoom * 0.7 };
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
            MapPointObjectKind::EVCharging => Some(("ev_station", Self::EV_STATION_SVG)),
            MapPointObjectKind::PopArea(..) => None,
        };
        if let Some(icon) = icon {
            let style_id = match poi.kind {
                MapPointObjectKind::TrainStation(is_train) => {
                    if is_train {
                        StyleId::new("train_station")
                    } else {
                        StyleId::new("railway_station")
                    }
                }
                MapPointObjectKind::TrafficLight => StyleId::new("poi_traffic_light"),
                MapPointObjectKind::EVCharging => StyleId::new("poi_ev_station"),
                MapPointObjectKind::Parking => StyleId::new("poi_parking"),
                MapPointObjectKind::Toilet => StyleId::new("poi_toilet"),
                _ => StyleId::new("poi"),
            };

            let background = if !matches!(poi.kind, MapPointObjectKind::TrafficLight) {
                Some(SvgBackground {
                    style_id: StyleId::new(format!("{}_icon_background",style_id.0)),
                    padding: 7.0 * dpi_scale,
                })
            } else {
                None
            };

            let icon_size = if matches!(poi.kind, MapPointObjectKind::TrafficLight) {
                33.0
            } else {
                30.0
            };

            geometry_data.push(GeometryData::Svg(SvgData {
                icon,
                position: DVec3::from((local_position.x, local_position.y, 0.0)),
                size: icon_size * dpi_scale,
                style_id: background.as_ref().map(|_| style_id),
                with_collision: true,
                background
            }));
        }

        if !poi.text.is_empty() {
            let id =
                hash(format!("{:?}{}{}", poi.text, local_position.x, local_position.y).as_bytes());
            let y_offset = if icon.is_some() { 30.0 } else { 0.0 };
            geometry_data.push(GeometryData::Text(TextData::new(
                id,
                poi.text.to_uppercase(),
                Vec2::new(0.0, y_offset * dpi_scale),
                27.0 * dpi_scale,
                LineData::new(vec![
                    DVec3::from((local_position.x, local_position.y, 0.0)),
                ])
            )));
        }
    }

    fn process_line(
        &self,
        geometry_data: &mut Vec<GeometryData>,
        line: LineString<f32>,
        interiors: Vec<LineString<f32>>,
        kind: MapGeomObjectKind,
        line_text_map: &mut HashMap<String, i32>,
        zoom_level: i32,
        dpi_scale: f32,
    ) {
        let zoom_level = MAX_ZOOM_LEVEL - zoom_level;
        let line = line.0;
        if line.len() >= 2 {
            let mut path_builder = Path::builder();
            path_builder.begin(point(line[0].x, line[0].y));

            for &p in line[1..].iter() {
                path_builder.line_to(point(p.x, p.y));
            }

            // fyi, we need to close the building path to properly build a closed stroke
            // also if interiors are not empty!
            let end_with_closing = matches!(kind, MapGeomObjectKind::Building {..}) || !interiors.is_empty();
            path_builder.end(end_with_closing);

            for interior in interiors {
                if let Some(first_point) = interior.0.first() {
                    path_builder.begin(point(first_point.x, first_point.y));

                    for p in interior.0.iter().skip(1) {
                        path_builder.line_to(point(p.x, p.y));
                    }

                    path_builder.end(true);
                }
            }

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
                                StyleId::new("rails"),
                                info.layer,
                                GeometryType::Polyline(PolylineOptions {
                                    width: 1.2 * zoom_level.max(1) as f32,
                                    ..Default::default()
                                }),
                                None,
                            ))
                        } else {
                            None
                        }
                    }
                },
                MapGeomObjectKind::AdminLine => {
                    (zoom_level >= 10).then(|| {
                        (
                            StyleId::new("admin_line"),
                            0,
                            GeometryType::Polyline(PolylineOptions {
                                width: 100.0 * zoom_level as f32,
                                ..Default::default()
                            }),
                            None,
                        )
                    })
                },
                MapGeomObjectKind::Nature(kind) => {
                    let style_id = match kind {
                        NatureKind::Ground => StyleId::new("ground"),
                        NatureKind::Park => StyleId::new("park"),
                        NatureKind::Forest => StyleId::new("forest"),
                        NatureKind::Water => StyleId::new("water"),
                    };
                    Some((style_id, -100, GeometryType::Polygon, None))
                }
                MapGeomObjectKind::Building(_) => {
                    Some((StyleId::new("building"), -99, GeometryType::Polygon, None))
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

                    geometry_data.push(GeometryData::Shape(ShapeData {
                        path: path_builder.clone().build(),
                        geometry_type: GeometryType::Polyline(PolylineOptions {
                            width: 0.8,
                            line_cap: LineCap::Butt,
                            line_join: LineJoin::Round,
                            tolerance: 0.02,
                        }),
                        style_id: StyleId::new("building_stand"),
                        index_layer_level: -99, // same as just buildings
                        styled_range_info: StyledRangeInfo(1, "skip"),
                    }));

                    geometry_data.push(GeometryData::ExtrudedPolygon(ExtrudedPolygonData {
                        path: path_builder.build(),
                        height: level as f32 * 2.0,
                    }));
                } else {
                    let double_style = match &kind {
                        MapGeomObjectKind::Nature(_) |
                        MapGeomObjectKind::Building(_) => { false },
                        _ => { zoom_level < 1 }
                    };
                    let tag = match &kind {
                        MapGeomObjectKind::Building(_) => { "skip" },
                        _ => { "" }
                    };

                    geometry_data.push(GeometryData::Shape(ShapeData {
                        path: path_builder.build(),
                        geometry_type,
                        style_id,
                        index_layer_level: layer_level as i8,
                        styled_range_info: StyledRangeInfo(if double_style { 0 } else { 1 }, tag),
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
                            geometry_data.push(GeometryData::Text(TextData::new(
                                hash(name.as_bytes()),
                                name.capitalize(),
                                Vec2::new(0.0, 0.0),
                                22.0 * dpi_scale,
                                LineData::new(line
                                    .iter()
                                    .map(|item| DVec3::new(item.x as f64, item.y as f64, 0.0))
                                    .collect())
                            )));
                        }
                    }
                }
            }
        }
    }
}
