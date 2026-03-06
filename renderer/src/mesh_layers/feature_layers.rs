use crate::global_context::GlobalContext;
use crate::mesh_layers::general_mesh_layer::GeneralMeshLayer;
use crate::mesh_layers::BaseMeshLayer;
use crate::pipelines::shape_pipeline::ShapePipeline;
use linked_hash_map::LinkedHashMap;
use wgpu::{CommandEncoder, RenderPass};

pub struct FeatureLayers {
    shape_layers: LinkedHashMap<String, GeneralMeshLayer<ShapePipeline>>,
}

#[derive(Default)]
pub struct FeatureLayerTag {
    pub name: &'static str,
    pub vertex_shader: Option<&'static str>,
    pub indirect: bool
}

impl FeatureLayers {
    pub fn new(tags: Vec<FeatureLayerTag>, global_context: &GlobalContext) -> FeatureLayers {
        let mut layers = FeatureLayers {
            shape_layers: LinkedHashMap::new(),
        };

        tags.into_iter().for_each(|tag| {
            let layer = GeneralMeshLayer::new(ShapePipeline::new(global_context, tag.vertex_shader, tag.indirect));
            layers.shape_layers.insert(tag.name.to_string(), layer);
        });

        layers
    }

    pub(crate) fn get_layer(&mut self, tag: &String) -> Option<&mut GeneralMeshLayer<ShapePipeline>> {
        self.shape_layers.get_mut(tag)
    }
}

impl BaseMeshLayer for FeatureLayers {
    fn prepare(&mut self, global_context: &GlobalContext) {
        self.shape_layers.iter_mut().for_each(|(_, layer)| {
            layer.prepare(global_context);
        });
    }

    fn update(&mut self, global_context: &mut GlobalContext) {
        self.shape_layers.iter_mut().for_each(|(_, layer)| {
            layer.update(global_context);
        });
    }

    fn compute(&mut self, encoder: &mut CommandEncoder, global_context: &mut GlobalContext) {
        self.shape_layers.iter_mut().for_each(|(_, layer)| {
            layer.compute(encoder, global_context);
        });
    }

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        self.shape_layers.iter_mut().for_each(|(_, layer)| {
            layer.render(render_pass, global_context);
        });
    }

    fn clear_by_key(&mut self, key: &str) {
        self.shape_layers
            .iter_mut()
            .for_each(|(_, layer)| layer.clear_by_key(key));
    }
}
