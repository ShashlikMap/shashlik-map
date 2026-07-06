use crate::tiles::mvt::mvt_parser::LocalMvtValue2;
use fast_mvt::{MvtFeatureRef, MvtLayerRef, MvtValue};
use osm::map::{
    HighwayKind, LayerKind, LineKind, MapGeomObject, MapGeomObjectKind, MapGeometry, WayInfo,
};
use std::collections::HashMap;

pub(crate) struct MvtSchemeParser {
    config: HashMap<&'static str, MvtPropHandler>,
}

impl MvtSchemeParser {
    pub fn new_map_tiler_v4() -> Self {
        let handlers = vec![MvtPropHandler::new("road", |handler| {
            let road_layer: i64 = handler.get_prop_value("layer");
            let road_class: String = handler.get_prop_value("class");
            let brunnel: String = handler.get_prop_value("brunnel");
            let brunnel: bool = !brunnel.is_empty();
            let ramp: bool = handler.get_prop_value("ramp");

            let mut highway_kind_name: Option<&str> = match road_class.as_str() {
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

            highway_kind_name.map(|highway_kind_name| {
                let mut bb = highway_kind_name.to_string();
                if ramp {
                    bb = format!("{highway_kind_name}_link");
                }
                let highway_kind = HighwayKind::from_descr(bb.as_str()).unwrap();
                MapGeomObject {
                    id: -1,
                    kind: MapGeomObjectKind::Way(WayInfo {
                        line_kind: LineKind::Highway { kind: highway_kind },
                        layer: if brunnel { road_layer as i32 } else { 0 },
                        layer_kind: LayerKind::None,
                        name_en: None,
                    }),
                }
            })
        })];
        Self::new_from_handlers(handlers)
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
        &mut self,
        layers: impl Iterator<Item = MvtLayerRef<'b>>,
        geom_builder: F,
    ) -> Vec<(MapGeomObject, MapGeometry<f32>)>
    where
        F: Fn(&MvtFeatureRef) -> Vec<MapGeometry<f32>>,
    {
        let mut res = vec![];
        for layer in layers {
            let handler = self.config.get_mut(layer.name());
            if let Some(handler) = handler {
                for feature in layer.features() {
                    let data = handler.build(&feature, &geom_builder);
                    res.extend(data);
                }
            }
        }

        res
    }
}

struct MvtPropHandler {
    layer: &'static str,
    builder: Box<dyn Fn(&Self) -> Option<MapGeomObject>>,
    map: HashMap<String, MvtValue>,
}

impl MvtPropHandler {
    pub fn new<F>(layer: &'static str, builder: F) -> Self
    where
        F: Fn(&Self) -> Option<MapGeomObject> + 'static,
    {
        Self {
            layer,
            builder: Box::new(builder),
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
    ) -> Vec<(MapGeomObject, MapGeometry<f32>)>
    where
        F: Fn(&MvtFeatureRef) -> Vec<MapGeometry<f32>>,
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
        T: From<LocalMvtValue2>,
    {
        self.map
            .get(key)
            // FIXME how to remove clone?
            .map(|value| LocalMvtValue2(value.clone()).into())
            .unwrap_or_default()
    }
}
