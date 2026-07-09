use indexmap::IndexMap;
use crate::global_context::GlobalContext;
use crate::mesh_layers::BaseMeshLayer;
use wgpu::{CommandEncoder, RenderPass};

pub struct FeatureLayers<ML: BaseMeshLayer> {
    feature_shape_layers: IndexMap<&'static str, ML>,
}

pub trait FeatureLayerTag {
    fn name(&self) -> &'static str;
}

pub struct NameLayerTag(pub &'static str);

impl FeatureLayerTag for NameLayerTag {
    fn name(&self) -> &'static str {
        self.0
    }
}

impl<ML: BaseMeshLayer> FeatureLayers<ML> {
    pub fn new<TAG: FeatureLayerTag, C>(tags: Vec<TAG>, mut ctor: C) -> FeatureLayers<ML>
    where
        C: FnMut(&TAG) -> ML,
    {
        let mut layers = FeatureLayers {
            feature_shape_layers: IndexMap::new(),
        };

        tags.into_iter().for_each(|tag| {
            let layer = ctor(&tag);
            layers.feature_shape_layers.insert(tag.name(), layer);
        });

        layers
    }

    pub(crate) fn get_layer(&mut self, tag: &str) -> Option<&mut ML> {
        self.feature_shape_layers.get_mut(tag)
    }
}

impl<ML: BaseMeshLayer> BaseMeshLayer for FeatureLayers<ML> {
    fn prepare(&mut self, global_context: &GlobalContext) {
        self.feature_shape_layers.iter_mut().for_each(|(_, layer)| {
            layer.prepare(global_context);
        });
    }

    fn update(&mut self, global_context: &mut GlobalContext) {
        self.feature_shape_layers.iter_mut().for_each(|(_, layer)| {
            layer.update(global_context);
        });
    }

    fn compute(&mut self, encoder: &mut CommandEncoder, global_context: &mut GlobalContext) {
        self.feature_shape_layers.iter_mut().for_each(|(_, layer)| {
            layer.compute(encoder, global_context);
        });
    }

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        self.feature_shape_layers.iter_mut().for_each(|(_, layer)| {
            layer.render(render_pass, global_context);
        });
    }

    fn clear_by_key(&mut self, key: &str) {
        self.feature_shape_layers
            .iter_mut()
            .for_each(|(_, layer)| layer.clear_by_key(key));
    }
}
