use crate::tiles::{
    ShashlikMapGeomObject,
};
use fast_mvt::{MvtFeatureRef, MvtLayerRef, MvtValue};
use log::error;
use osm::map::{MapGeometry};
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) struct MvtSchemeParser {
    config: HashMap<&'static str, MvtPropHandler>,
}

impl MvtSchemeParser {

    pub(crate) fn new_from_handlers(handlers: Vec<MvtPropHandler>) -> Self {
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
    ) -> Vec<(ShashlikMapGeomObject, MapGeometry<i32>)>
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
pub(crate) struct MvtPropHandler {
    layer: &'static str,
    builder: Arc<dyn Fn(&Self) -> Option<ShashlikMapGeomObject> + Send + Sync>,
    map: HashMap<String, MvtValue>,
}

impl MvtPropHandler {
    pub fn new<F>(layer: &'static str, builder: F) -> Self
    where
        F: Fn(&Self) -> Option<ShashlikMapGeomObject> + Send + Sync + 'static,
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
    ) -> Vec<(ShashlikMapGeomObject, MapGeometry<i32>)>
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
            .map(|mut geom_obj| {
                // TODO No ID later
                geom_obj.id = feature.id().unwrap_or(0) as i64;
                let geom = geom_builder(feature);
                geom.into_iter()
                    .map(|geometry| (geom_obj.clone(), geometry))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn get_prop_value<T: Default>(&self, key: &'static str) -> T
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

pub(crate) struct LocalMvtValue<'a>(pub &'a MvtValue);

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
