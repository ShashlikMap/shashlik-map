use crate::tiles::mvt::mvt_parser::LocalMvtValue2;
use fast_mvt::{MvtFeatureRef, MvtLayerRef, MvtValue};
use osm::map::{MapGeomObject, MapGeometry};
use std::collections::HashMap;

pub(crate) struct MvtSchemeParser {
    config: HashMap<&'static str, MvtPropHandler<'static>>,
}

impl MvtSchemeParser {
    pub fn new_map_tiler_v4() -> Self {
        let handlers = vec![MvtPropHandler::new("road", |handler| {
            let layer: Option<i64> = handler.get_prop_value("layer");
            let road_class: Option<String> = handler.get_prop_value("class");
            let brunnel: Option<bool> = handler.get_prop_value("brunnel");
            let ramp: Option<bool> = handler.get_prop_value("ramp");

            

            todo!("MapGeomObject impl");
            // true
        })];
        Self::new_from_handlers(handlers)
    }

    fn new_from_handlers(handlers: Vec<MvtPropHandler<'static>>) -> Self {
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

struct MvtPropHandler<'a> {
    layer: &'static str,
    builder: Box<dyn Fn(&Self) -> MapGeomObject>,
    map: HashMap<String, MvtValue>,
}

impl<'a> MvtPropHandler<'a> {
    pub fn new<F>(layer: &'static str, builder: F) -> Self
    where
        F: Fn(&Self) -> MapGeomObject + 'static,
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
        let geom = geom_builder(feature);
        geom.into_iter()
            .map(|geometry| (geom_obj.clone(), geometry))
            .collect()
    }

    pub fn get_prop_value<T>(&self, key: &'static str) -> Option<T>
    where
        T: From<LocalMvtValue2>,
    {
        self.map
            .get(key)
            .map(|value| LocalMvtValue2(value.clone()).into()) // TODO how to remove clone?
    }
}
