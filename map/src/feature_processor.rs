use crate::tiles::default_tiles_provider::FeatureProcessor;
use geo_types::{Coord, LineString};
use glam::{DVec3, Vec2};
use lyon::geom::point;
use lyon::path::{Path, Winding};
use osm::map::{
    HighwayKind, LayerKind, LineKind,
};
use renderer_common::geometry_data::{ExtrudedPolygonData, GeometryData, GeometryType, LineData, PolylineOptions, ShapeData, StyledRangeInfo, IconBackground, IconShapeData, TextData, IconData};
use renderer_common::style_id::StyleId;
use capitalize::Capitalize;
use geo::Scale;
use lyon::algorithms::measure::{PathMeasurements, SampleType};
use lyon::geom::euclid::{point2, Box2D};
use lyon::lyon_tessellation::{LineCap, LineJoin};
use lyon::path::builder::BorderRadii;
use rand::RngExt;
use renderer_common::geometry_data::IconType::SvgBinary;
use crate::MAX_ZOOM_LEVEL;
use crate::tiles::{ShashlikMapGeomObjectKind, ShashlikMapLinearRef, ShashlikMapPointInfo, ShashlikMapPointObjectKind, ShashlikNatureKind};

pub struct ShashlikFeatureProcessor {
    include_extruded: bool,
    data_filter: fn(zoom_level: i32, &ShashlikMapGeomObjectKind) -> bool,
}

impl Default for ShashlikFeatureProcessor {
    fn default() -> Self {
        ShashlikFeatureProcessor::new(true, |zoom_level, kind| {
            // by default, we keep building only for zoom_level >= 13 and TrafficLight/Toilet for >= 15(due to POI id issues)
            match kind {
                ShashlikMapGeomObjectKind::Poi(info) => {
                    match info.kind {
                        ShashlikMapPointObjectKind::TrafficLight | ShashlikMapPointObjectKind::Toilet => zoom_level >= 15,
                        _ => true,
                    }
                }
                ShashlikMapGeomObjectKind::Building(_) => zoom_level >= 13,
                _ => true,
            }
        })
    }
}

impl ShashlikFeatureProcessor {
    const TRAFFIC_LIGHT_SVG: &'static [u8] = include_bytes!("../svg/traffic_light.svg");
    const PARKING_SVG: &'static [u8] = include_bytes!("../svg/parking.svg");
    const TOILETS_SVG: &'static [u8] = include_bytes!("../svg/toilet.svg");
    const TRAIN_STATION_SVG: &'static [u8] = include_bytes!("../svg/train_station.svg");
    const EV_STATION_SVG: &'static [u8] = include_bytes!("../svg/ev_station.svg");
    // const CROSSING_SVG: &'static [u8] = include_bytes!("../svg/pedestrian-crossing.svg");
    pub fn new(include_extruded: bool,
               data_filter: fn(zoom_level: i32, kind: &ShashlikMapGeomObjectKind) -> bool) -> Self {
        ShashlikFeatureProcessor {
            include_extruded,
            data_filter,
        }
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
        mut id: i64,
        geometry_data: &mut Vec<GeometryData>,
        poi: &ShashlikMapPointInfo,
        zoom_level: i32,
        local_position: &Coord,
        dpi_scale: f32,
    ) {
        if !(self.data_filter)(zoom_level, &ShashlikMapGeomObjectKind::Poi(poi.clone())) {
            return;
        }
        // Temporary workaround for POIs without IDs
        // For some reason, MapTiler doesn't return it for every POI.
        // But the collision detector needs it.
        if id == 0 {
            let mut rng = rand::rng();
            id = rng.random();
        }
        let icon: Option<(&str, &[u8])> = match poi.kind {
            ShashlikMapPointObjectKind::TrainStation(is_train) => {
                if is_train {
                    Some(("train_station", Self::TRAIN_STATION_SVG))
                } else {
                    Some(("railway_station", Self::TRAIN_STATION_SVG))
                }
            }
            ShashlikMapPointObjectKind::TrafficLight => Some(("traffic_light", Self::TRAFFIC_LIGHT_SVG)),
            ShashlikMapPointObjectKind::Toilet => Some(("toilets", Self::TOILETS_SVG)),
            ShashlikMapPointObjectKind::Parking => Some(("parking", Self::PARKING_SVG)),
            ShashlikMapPointObjectKind::EVCharging => Some(("ev_station", Self::EV_STATION_SVG)),
            ShashlikMapPointObjectKind::PopArea(..) => None,
            // we skip crossing since we need only to calculate linear refs, but it happens before,
            // in future we should have it at all as a POI representation
            ShashlikMapPointObjectKind::Crossing => None,
        };
        if let Some(icon) = icon {
            let style_id = match poi.kind {
                ShashlikMapPointObjectKind::TrainStation(is_train) => {
                    if is_train {
                        Some(StyleId::new("train_station"))
                    } else {
                        Some(StyleId::new("railway_station"))
                    }
                }
                ShashlikMapPointObjectKind::TrafficLight | ShashlikMapPointObjectKind::Crossing => None,
                ShashlikMapPointObjectKind::EVCharging => Some(StyleId::new("poi_ev_station")),
                ShashlikMapPointObjectKind::Parking => Some(StyleId::new("poi_parking")),
                ShashlikMapPointObjectKind::Toilet => Some(StyleId::new("poi_toilet")),
                _ => Some(StyleId::new("poi")),
            };

            let icon_size = if matches!(poi.kind, ShashlikMapPointObjectKind::TrafficLight) {
                33.0
            } else {
                30.0
            } * dpi_scale;

            let background = style_id.as_ref().map(|style_id| {
                let padding = 7.0 * dpi_scale;
                IconBackground {
                    style_id: StyleId::new(format!("{}_icon_background", style_id.0)),
                    shape: Box::new(move |data: &IconShapeData| -> Path {
                        let mut builder = Path::builder();
                        let half_size = padding + data.size / 2.0;
                        let rect =
                            Box2D::new(point2(-half_size, -half_size), point2(half_size, half_size));
                        builder.add_rounded_rectangle(
                            &rect,
                            &BorderRadii::new(10.0),
                            Winding::Positive,
                        );
                        builder.build()
                    }),
                }
            });

            let icon_data = IconData {
                id: icon.0,
                icon_type: SvgBinary(style_id, icon.1),
            };
            geometry_data.push(GeometryData::Svg(IconShapeData {
                id: id as u64,
                icon_data,
                position: DVec3::from((local_position.x, local_position.y, 0.0)),
                size: icon_size,
                with_collision: true,
                background
            }));
        }

        if !poi.text.is_empty() {
            let y_offset = if icon.is_some() { 30.0 } else { 0.0 };
            let id = id as u64;
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
        id: i64,
        geometry_data: &mut Vec<GeometryData>,
        mut line: LineString<f32>,
        interiors: Vec<LineString<f32>>,
        kind: ShashlikMapGeomObjectKind,
        zoom_level: i32,
        dpi_scale: f32,
    ) {
        let zoom_level = MAX_ZOOM_LEVEL - zoom_level;
        if line.0.len() >= 2 {
            if !(self.data_filter)(MAX_ZOOM_LEVEL - zoom_level, &kind) {
                return;
            }
            if let Some((style_id,
                            layer_level,
                            geometry_type,
                            name)) = match &kind {
                ShashlikMapGeomObjectKind::Way(info) => match info.line_kind {
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
                    LineKind::Label => {
                        info.name_en.as_ref().map(|name| {
                            (
                                // so far there is no styling for text
                                StyleId::new("no_style_label"),
                                info.layer,
                                GeometryType::Polyline(PolylineOptions {
                                    // 0 width to drop rendering processing
                                    width: 0f32,
                                    ..Default::default()
                                }),
                                Some(name.clone()),
                            )
                        })
                    }
                },
                ShashlikMapGeomObjectKind::AdminLine => {
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
                ShashlikMapGeomObjectKind::Nature(kind) => {
                    let style_id = match kind {
                        ShashlikNatureKind::Ground => StyleId::new("ground"),
                        ShashlikNatureKind::Park => StyleId::new("park"),
                        ShashlikNatureKind::Forest => StyleId::new("forest"),
                        ShashlikNatureKind::Water => StyleId::new("water"),
                    };
                    Some((style_id, -100, GeometryType::Polygon, None))
                }
                ShashlikMapGeomObjectKind::Building(_) => {
                    Some((StyleId::new("building"), -98, GeometryType::Polygon, None))
                }
                _ => None,
            } {
                // a small trick to get rid of many coplanar walls issues
                // fyi, it might be better to skip it for CPU only devices
                if matches!(kind, ShashlikMapGeomObjectKind::Building(_)) {
                    if line.0.len() % 2 == 0 {
                        line.scale_mut(1.01);
                    }
                }
                let mut path_builder = Path::builder();
                path_builder.begin(point(line[0].x, line[0].y));

                for &p in line[1..].iter() {
                    path_builder.line_to(point(p.x, p.y));
                }

                // fyi, we need to close the building path to properly build a closed stroke
                // also if interiors are not empty!
                let end_with_closing = matches!(kind, ShashlikMapGeomObjectKind::Building(_)) || !interiors.is_empty();
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

                if let ShashlikMapGeomObjectKind::Building(level) = kind {
                    let building_path = path_builder.build();

                    let mut styled_range_info = StyledRangeInfo::new(1, true);
                    if zoom_level == 0 && self.include_extruded {
                        let level = if level == 0 {
                            let (point, _) = building_path.first_endpoint().unwrap_or_default();
                            (((point.x.abs() as i32 * point.y.abs() as i32) % 2) + 2) as u16
                        } else {
                            level
                        };

                        styled_range_info.scale_filter = Some(|scale| scale <= 0.8f32);
                        geometry_data.push(GeometryData::Shape(ShapeData {
                            path: building_path.clone(),
                            geometry_type: GeometryType::Polyline(PolylineOptions {
                                width: 0.8,
                                line_cap: LineCap::Butt,
                                line_join: LineJoin::Round,
                                tolerance: 0.02,
                            }),
                            style_id: StyleId::new("building_stand"),
                            index_layer_level: -99,
                            styled_range_info: styled_range_info.clone(),
                        }));


                        geometry_data.push(GeometryData::ExtrudedPolygon(ExtrudedPolygonData {
                            path: building_path.clone(),
                            height: level as f32 * 2.0,
                        }));
                    }

                    styled_range_info.scale_filter = Some(|scale| scale >= 1.8f32);
                    geometry_data.push(GeometryData::Shape(ShapeData {
                        path: building_path,
                        geometry_type,
                        style_id,
                        index_layer_level: layer_level as i16,
                        styled_range_info,
                    }));
                } else {
                    let double_style = match &kind {
                        ShashlikMapGeomObjectKind::Building(_) => {
                            panic!("Buildings should not be processed here");
                        }
                        ShashlikMapGeomObjectKind::Nature(_) => {
                            false
                        }
                        ShashlikMapGeomObjectKind::Way(info) => {
                            match info.line_kind {
                                LineKind::Highway { .. } => { zoom_level < 1 }
                                _ => { false }
                            }
                        }
                        _ => { zoom_level < 1 }
                    };

                    // creating a bucket(10 sub-layers) for each layer so we can add some associated data atop of the layer.
                    // we need to add markings, but it's not possible now to mix different instance_offset for the same layer
                    let layer_level= if layer_level >= 0 {
                        layer_level * 10
                    } else {
                        layer_level
                    } as i16;

                    let path = path_builder.build();

                    geometry_data.push(GeometryData::Shape(ShapeData {
                        path: path.clone(),
                        geometry_type,
                        style_id,
                        index_layer_level: layer_level,
                        styled_range_info: StyledRangeInfo::new(if double_style { 0 } else { 1 }, false),
                    }));

                    match &kind {
                        ShashlikMapGeomObjectKind::Way(info) => {
                            if !info.linear_refs.is_empty() {
                                let path_measure = PathMeasurements::from_path(&path, 1.0);
                                let mut path_sampler = path_measure.create_sampler(&path, SampleType::Normalized);
                                info.linear_refs.iter().cloned().for_each(|linear_ref| {
                                    match linear_ref {
                                        ShashlikMapLinearRef::Crossing(s_value) => {
                                            let mut builder = Path::builder();
                                            let len = 0.65 * (1.0 / path_measure.length());

                                            // don't exceed start
                                            let start = if s_value - len < 0.0 {
                                                len
                                            } else {
                                                s_value - len
                                            };
                                            // don't exceed end
                                            let end = if s_value + len > 1.0 {
                                                len - s_value
                                            } else {
                                                s_value + len
                                            };
                                            path_sampler.split_range(start..end, &mut builder);
                                            let temp_path = builder.build();

                                            geometry_data.push(GeometryData::Shape(ShapeData {
                                                path: temp_path,
                                                geometry_type,
                                                style_id: StyleId::new("crossing_mark"),
                                                index_layer_level: layer_level + 1, // one layer up than road layer
                                                styled_range_info: StyledRangeInfo::new(1, true),
                                            }));
                                        }
                                    }
                                });
                            }
                        }
                        _ => {}
                    }
                }

                if let Some(name) = name {
                    geometry_data.push(GeometryData::Text(TextData::new(
                        id as u64,
                        name.capitalize(),
                        Vec2::new(0.0, 0.0),
                        22.0 * dpi_scale,
                        LineData::new(line
                            .into_iter()
                            .map(|item| DVec3::new(item.x as f64, item.y as f64, 0.0))
                            .collect())
                    )));
                }
            }
        }
    }
}
