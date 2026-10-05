use crate::tiles::parsers::mvt_scheme_parser::{MvtPropHandler, MvtSchemeParser};
use crate::tiles::{
    ShashlikMapGeomObject, ShashlikMapGeomObjectKind, ShashlikMapPointInfo,
    ShashlikMapPointObjectKind, ShashlikNatureKind, ShashlikPopAreaInfo, ShashlikWayInfo,
};
use osm::map::{HighwayKind, LayerKind, LineKind, RailwayKind};

impl MvtSchemeParser {
    pub fn new_map_tiler_v4() -> Self {
        let road_handler =
            MvtPropHandler::new("road", |handler| {
                let road_layer: i64 = handler.get_prop_value("layer");
                let road_class: String = handler.get_prop_value("class");
                let brunnel: String = handler.get_prop_value("brunnel");
                let brunnel: bool = !brunnel.is_empty();
                let ramp: bool = handler.get_prop_value("ramp");

                let highway_kind_name: Option<&str> = match road_class.as_str() {
                    "motorway" => Some("motorway"),
                    "primary" => Some("primary"),
                    "secondary" => Some("secondary"),
                    "tertiary" => Some("tertiary"),
                    "unclassified" => Some("unclassified"),
                    "residential" => Some("residential"),
                    "minor" => Some("residential"),
                    "service" => Some("service"),
                    "trunk" => Some("trunk"),
                    _ => None,
                };

                highway_kind_name.and_then(|highway_kind_name| {
                    let mut highway_tag = highway_kind_name.to_string();
                    if ramp {
                        highway_tag = format!("{highway_kind_name}_link");
                    }
                    HighwayKind::from_descr(highway_tag.as_str()).map(|kind| {
                        ShashlikMapGeomObject {
                            id: -1,
                            kind: ShashlikMapGeomObjectKind::Way(ShashlikWayInfo {
                                line_kind: LineKind::Highway { kind },
                                layer: if brunnel { road_layer as i32 } else { 0 },
                                layer_kind: LayerKind::None,
                                name_en: None,
                                linear_refs: vec![],
                            }),
                        }
                    })
                })
            });

        let road_label_handler = crate::tiles::parsers::mvt_scheme_parser::MvtPropHandler::new(
            "road_label",
            |handler| {
                let name_en: String = handler.get_prop_value("name:en");
                let name: String = handler.get_prop_value("name");

                Some(ShashlikMapGeomObject {
                    id: -1,
                    kind: ShashlikMapGeomObjectKind::Way(ShashlikWayInfo {
                        line_kind: LineKind::Label,
                        layer: 0,
                        layer_kind: LayerKind::None,
                        name_en: Some(if name_en.is_empty() { name } else { name_en }),
                        linear_refs: vec![],
                    }),
                })
            },
        );

        let water_handler =
            crate::tiles::parsers::mvt_scheme_parser::MvtPropHandler::new("water", |_| {
                Some(ShashlikMapGeomObject {
                    id: -1,
                    kind: ShashlikMapGeomObjectKind::Nature(ShashlikNatureKind::Water),
                })
            });

        let forest_handler =
            crate::tiles::parsers::mvt_scheme_parser::MvtPropHandler::new("forest", |_| {
                Some(ShashlikMapGeomObject {
                    id: -1,
                    kind: ShashlikMapGeomObjectKind::Nature(ShashlikNatureKind::Forest),
                })
            });

        let wood_handler =
            crate::tiles::parsers::mvt_scheme_parser::MvtPropHandler::new("wood", |_| {
                Some(ShashlikMapGeomObject {
                    id: -1,
                    kind: ShashlikMapGeomObjectKind::Nature(ShashlikNatureKind::Forest),
                })
            });

        let grass_handler =
            crate::tiles::parsers::mvt_scheme_parser::MvtPropHandler::new("grass", |_| {
                Some(ShashlikMapGeomObject {
                    id: -1,
                    kind: ShashlikMapGeomObjectKind::Nature(ShashlikNatureKind::Park),
                })
            });

        let building_handler =
            crate::tiles::parsers::mvt_scheme_parser::MvtPropHandler::new("building", |handler| {
                // TODO skip for certain zoom levels
                let height: i64 = handler.get_prop_value("height");
                // fyi, so far we don't support
                let height_min: i64 = handler.get_prop_value("height_min");
                let underground: bool = handler.get_prop_value("underground");
                (!underground && height_min == 0).then_some(ShashlikMapGeomObject {
                    id: -1,
                    // fyi, 3 - koef to convert map tiler height to osm levels, 2 - feature processor multiplier
                    kind: ShashlikMapGeomObjectKind::Building(
                        ((height / (3 * 2)) as u16).clamp(0, 100),
                    ),
                })
            });

        let street_furniture = crate::tiles::parsers::mvt_scheme_parser::MvtPropHandler::new(
            "street_furniture",
            |handler| {
                let class: String = handler.get_prop_value("class");
                let subclass: String = handler.get_prop_value("subclass");

                match (class.as_str(), subclass.as_str()) {
                    ("street", "toilets") => Some(ShashlikMapPointObjectKind::Toilet),
                    ("street", "traffic_signals") => Some(ShashlikMapPointObjectKind::TrafficLight),
                    ("street", "crossing") => Some(ShashlikMapPointObjectKind::Crossing),
                    _ => None,
                }
                .map(|kind| ShashlikMapGeomObject {
                    id: -1,
                    kind: ShashlikMapGeomObjectKind::Poi(ShashlikMapPointInfo {
                        text: "".to_string(),
                        kind,
                    }),
                })
            },
        );

        let poi_station = crate::tiles::parsers::mvt_scheme_parser::MvtPropHandler::new(
            "poi_station",
            |handler| {
                let agg_stop: bool = handler.get_prop_value("agg_stop");
                let class: String = handler.get_prop_value("class");
                let subclass: String = handler.get_prop_value("subclass");
                let name: String = handler.get_prop_value("name:en");

                match (agg_stop, class.as_str(), subclass.as_str()) {
                    (true, "railway", "station") => {
                        Some(ShashlikMapPointObjectKind::TrainStation(true))
                    }
                    (true, "railway", "subway") => {
                        Some(ShashlikMapPointObjectKind::TrainStation(false))
                    }
                    _ => None,
                }
                .map(|kind| ShashlikMapGeomObject {
                    id: -1,
                    kind: ShashlikMapGeomObjectKind::Poi(ShashlikMapPointInfo { text: name, kind }),
                })
            },
        );

        let poi_transport_handler = crate::tiles::parsers::mvt_scheme_parser::MvtPropHandler::new(
            "poi_transport",
            |handler| {
                let class: String = handler.get_prop_value("class");
                let subclass: String = handler.get_prop_value("subclass");

                match (class.as_str(), subclass.as_str()) {
                    ("parking", "parking") => Some(ShashlikMapPointObjectKind::Parking),
                    ("fuel", "charging_station") => Some(ShashlikMapPointObjectKind::EVCharging),
                    _ => None,
                }
                .map(|kind| ShashlikMapGeomObject {
                    id: -1,
                    kind: ShashlikMapGeomObjectKind::Poi(ShashlikMapPointInfo {
                        text: "".to_string(),
                        kind,
                    }),
                })
            },
        );

        let city_country_label_handler =
            |handler: &crate::tiles::parsers::mvt_scheme_parser::MvtPropHandler| {
                let name_en: String = handler.get_prop_value("name:en");
                let name: String = handler.get_prop_value("name");

                Some(ShashlikMapGeomObject {
                    id: -1,
                    kind: ShashlikMapGeomObjectKind::Poi(ShashlikMapPointInfo {
                        text: if name_en.is_empty() { name } else { name_en },
                        kind: ShashlikMapPointObjectKind::PopArea(ShashlikPopAreaInfo {
                            level: 0,
                            population: 0,
                        }),
                    }),
                })
            };
        let city_label_handler = crate::tiles::parsers::mvt_scheme_parser::MvtPropHandler::new(
            "city_label",
            city_country_label_handler,
        );
        let country_label_handler = crate::tiles::parsers::mvt_scheme_parser::MvtPropHandler::new(
            "country_label",
            city_country_label_handler,
        );

        let country_border_handler = crate::tiles::parsers::mvt_scheme_parser::MvtPropHandler::new(
            "country_border",
            |handler| {
                let maritime: bool = handler.get_prop_value("maritime");
                (!maritime).then_some(ShashlikMapGeomObject {
                    id: -1,
                    kind: ShashlikMapGeomObjectKind::AdminLine,
                })
            },
        );

        let railway_handler =
            crate::tiles::parsers::mvt_scheme_parser::MvtPropHandler::new("railway", |handler| {
                let class: String = handler.get_prop_value("class");
                (class == "rail" || class == "monorail").then_some(ShashlikMapGeomObject {
                    id: -1,
                    kind: ShashlikMapGeomObjectKind::Way(ShashlikWayInfo {
                        line_kind: LineKind::Railway {
                            kind: RailwayKind::Rail,
                        },
                        layer: 0,
                        layer_kind: LayerKind::None,
                        name_en: None,
                        linear_refs: vec![],
                    }),
                })
            });

        Self::new_from_handlers(vec![
            road_handler,
            road_label_handler,
            railway_handler,
            water_handler,
            building_handler,
            forest_handler,
            wood_handler,
            grass_handler,
            street_furniture,
            poi_station,
            poi_transport_handler,
            city_label_handler,
            country_label_handler,
            country_border_handler,
        ])
    }
}
