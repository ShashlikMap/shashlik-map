use fast_mvt::{MvtFeatureRef, MvtLayerRef, MvtValue};
use log::error;
use osm::map::{
    HighwayKind, LayerKind, LineKind, MapGeomObject, MapGeomObjectKind, MapGeometry, MapPointInfo,
    MapPointObjectKind, NatureKind, PopAreaInfo, RailwayKind, WayInfo,
};
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) struct MvtSchemeParser {
    config: HashMap<&'static str, MvtPropHandler>,
}

impl MvtSchemeParser {
    pub fn new_map_tiler_v4() -> Self {
        let road_handler = MvtPropHandler::new("road", |handler| {
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
                HighwayKind::from_descr(highway_tag.as_str()).map(|kind| MapGeomObject {
                    id: -1,
                    kind: MapGeomObjectKind::Way(WayInfo {
                        line_kind: LineKind::Highway { kind },
                        layer: if brunnel { road_layer as i32 } else { 0 },
                        layer_kind: LayerKind::None,
                        name_en: None,
                    }),
                })
            })
        });

        let road_label_handler = MvtPropHandler::new("road_label", |handler| {
            let name_en: String = handler.get_prop_value("name:en");
            let name: String = handler.get_prop_value("name");

            Some(MapGeomObject {
                id: -1,
                kind: MapGeomObjectKind::Way(WayInfo {
                    line_kind: LineKind::Label,
                    layer: 0,
                    layer_kind: LayerKind::None,
                    name_en: Some(if name_en.is_empty() { name } else { name_en }),
                }),
            })
        });

        let water_handler = MvtPropHandler::new("water", |_| {
            Some(MapGeomObject {
                id: -1,
                kind: MapGeomObjectKind::Nature(NatureKind::Water),
            })
        });

        let forest_handler = MvtPropHandler::new("forest", |_| {
            Some(MapGeomObject {
                id: -1,
                kind: MapGeomObjectKind::Nature(NatureKind::Forest),
            })
        });

        let wood_handler = MvtPropHandler::new("wood", |_| {
            Some(MapGeomObject {
                id: -1,
                kind: MapGeomObjectKind::Nature(NatureKind::Forest),
            })
        });

        let grass_handler = MvtPropHandler::new("grass", |_| {
            Some(MapGeomObject {
                id: -1,
                kind: MapGeomObjectKind::Nature(NatureKind::Park),
            })
        });

        let building_handler = MvtPropHandler::new("building", |handler| {
            // TODO skip for certain zoom levels
            let height: i64 = handler.get_prop_value("height");
            let underground: bool = handler.get_prop_value("underground");
            (!underground).then_some(MapGeomObject {
                id: -1,
                // fyi, 3 - koef to convert map tiler height to osm levels, 2 - feature processor multiplier
                kind: MapGeomObjectKind::Building(((height / (3 * 2)) as u16).clamp(0, 100)),
            })
        });

        let street_furniture = MvtPropHandler::new("street_furniture", |handler| {
            let class: String = handler.get_prop_value("class");
            let subclass: String = handler.get_prop_value("subclass");

            match (class.as_str(), subclass.as_str()) {
                ("street", "toilets") => Some(MapPointObjectKind::Toilet),
                ("street", "traffic_signals") => Some(MapPointObjectKind::TrafficLight),
                _ => None,
            }
            .map(|kind| MapGeomObject {
                id: -1,
                kind: MapGeomObjectKind::Poi(MapPointInfo {
                    text: "".to_string(),
                    kind,
                }),
            })
        });

        let poi_station = MvtPropHandler::new("poi_station", |handler| {
            let agg_stop: bool = handler.get_prop_value("agg_stop");
            let class: String = handler.get_prop_value("class");
            let subclass: String = handler.get_prop_value("subclass");
            let name: String = handler.get_prop_value("name:en");

            match (agg_stop, class.as_str(), subclass.as_str()) {
                (true, "railway", "station") => Some(MapPointObjectKind::TrainStation(true)),
                (true, "railway", "subway") => Some(MapPointObjectKind::TrainStation(false)),
                _ => None,
            }
            .map(|kind| MapGeomObject {
                id: -1,
                kind: MapGeomObjectKind::Poi(MapPointInfo { text: name, kind }),
            })
        });

        let poi_transport_handler = MvtPropHandler::new("poi_transport", |handler| {
            let class: String = handler.get_prop_value("class");
            let subclass: String = handler.get_prop_value("subclass");

            match (class.as_str(), subclass.as_str()) {
                ("parking", "parking") => Some(MapPointObjectKind::Parking),
                ("fuel", "charging_station") => Some(MapPointObjectKind::EVCharging),
                _ => None,
            }
            .map(|kind| MapGeomObject {
                id: -1,
                kind: MapGeomObjectKind::Poi(MapPointInfo {
                    text: "".to_string(),
                    kind,
                }),
            })
        });

        let city_country_label_handler = |handler: &MvtPropHandler| {
            let name_en: String = handler.get_prop_value("name:en");
            let name: String = handler.get_prop_value("name");

            Some(MapGeomObject {
                id: -1,
                kind: MapGeomObjectKind::Poi(MapPointInfo {
                    text: if name_en.is_empty() { name } else { name_en },
                    kind: MapPointObjectKind::PopArea(PopAreaInfo {
                        level: 0,
                        population: 0,
                    }),
                }),
            })
        };
        let city_label_handler = MvtPropHandler::new("city_label", city_country_label_handler);
        let country_label_handler =
            MvtPropHandler::new("country_label", city_country_label_handler);

        let country_border_handler = MvtPropHandler::new("country_border", |handler| {
            let maritime: bool = handler.get_prop_value("maritime");
            (!maritime).then_some(MapGeomObject {
                id: -1,
                kind: MapGeomObjectKind::AdminLine,
            })
        });

        let railway_handler = MvtPropHandler::new("railway", |handler| {
            let class: String = handler.get_prop_value("class");
            (class == "rail" || class == "monorail").then_some(MapGeomObject {
                id: -1,
                kind: MapGeomObjectKind::Way(WayInfo {
                    line_kind: LineKind::Railway {
                        kind: RailwayKind::Rail,
                    },
                    layer: 0,
                    layer_kind: LayerKind::None,
                    name_en: None,
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

    fn new_from_handlers(handlers: Vec<MvtPropHandler>) -> Self {
        Self {
            config: handlers
                .into_iter()
                .map(|item| (item.layer(), item))
                .collect(),
        }
    }

    pub fn parse<'b, F>(
        &self,
        layers: impl Iterator<Item = MvtLayerRef<'b>>,
        geom_builder: F,
    ) -> Vec<(MapGeomObject, MapGeometry<i32>)>
    where
        F: Fn(&MvtFeatureRef) -> Vec<MapGeometry<i32>>,
    {
        let geom_builder_ref = &geom_builder;
        layers
            .filter_map(|layer| {
                self.config
                    .get(layer.name())
                    .cloned()
                    .map(|handler| (layer, handler))
            })
            .flat_map(|(layer, mut handler)| {
                layer
                    .features()
                    .flat_map(move |feature| handler.build(&feature, geom_builder_ref))
            })
            .collect()
    }
}

#[derive(Clone)]
struct MvtPropHandler {
    layer: &'static str,
    builder: Arc<dyn Fn(&Self) -> Option<MapGeomObject> + Send + Sync>,
    map: HashMap<String, MvtValue>,
}

impl MvtPropHandler {
    pub fn new<F>(layer: &'static str, builder: F) -> Self
    where
        F: Fn(&Self) -> Option<MapGeomObject> + Send + Sync + 'static,
    {
        Self {
            layer,
            builder: Arc::new(builder),
            map: HashMap::new(),
        }
    }

    pub fn layer(&self) -> &'static str {
        self.layer
    }

    pub fn build<F>(
        &mut self,
        feature: &MvtFeatureRef<'_>,
        geom_builder: &F,
    ) -> Vec<(MapGeomObject, MapGeometry<i32>)>
    where
        F: Fn(&MvtFeatureRef) -> Vec<MapGeometry<i32>>,
    {
        self.map.clear();
        for property in feature.properties() {
            if let Ok((key, value)) = property {
                self.map.insert(key.to_string(), value.into_owned());
            }
        }

        let geom_obj = (self.builder)(&self);
        geom_obj
            .map(|geom_obj| {
                let geom = geom_builder(feature);
                geom.into_iter()
                    .map(|geometry| (geom_obj.clone(), geometry))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_prop_value<T: Default>(&self, key: &'static str) -> T
    where
        for<'a> Option<T>: From<LocalMvtValue<'a>>,
    {
        self.map
            .get(key)
            // How to get rid of clone()?
            .and_then(|value| LocalMvtValue(value).into())
            .unwrap_or_default()
    }
}

struct LocalMvtValue<'a>(pub &'a MvtValue);

impl LocalMvtValue<'_> {
    fn unexpected_type<T>(&self, expected: &str) -> Option<T> {
        error!("Unexpected {} MvtValue: {:?}", expected, self.0);
        None
    }
}
impl From<LocalMvtValue<'_>> for Option<i64> {
    fn from(value: LocalMvtValue) -> Self {
        match value.0 {
            MvtValue::SInt(value) => Some(*value),
            _ => value.unexpected_type("i64"),
        }
    }
}

impl From<LocalMvtValue<'_>> for Option<String> {
    fn from(value: LocalMvtValue) -> Self {
        match value.0 {
            MvtValue::String(value) => Some(value.clone()),
            _ => value.unexpected_type("String"),
        }
    }
}

impl From<LocalMvtValue<'_>> for Option<bool> {
    fn from(value: LocalMvtValue) -> Self {
        match value.0 {
            MvtValue::Bool(value) => Some(*value),
            _ => value.unexpected_type("bool"),
        }
    }
}
