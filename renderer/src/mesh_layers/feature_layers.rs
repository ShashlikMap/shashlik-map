use crate::mesh_layers::general_mesh_layer::GeneralMeshLayer;
use crate::mesh_layers::BaseMeshLayer;
use crate::pipelines::shape_pipeline::ShapePipeline;
use crate::styles::style_store::StyleStore;
use crate::GlobalContext;
use linked_hash_map::LinkedHashMap;
use wgpu::{Device, Queue, RenderPass, SurfaceConfiguration};

pub struct FeatureLayers {
    shape_layers: LinkedHashMap<String, GeneralMeshLayer<ShapePipeline>>,
}

impl FeatureLayers {
    pub fn new(tags: &[String], device: &Device, global_context: &mut GlobalContext, style_store: &StyleStore) -> FeatureLayers {
        let mut layers = FeatureLayers {
            shape_layers: LinkedHashMap::new(),
        };

        tags.into_iter().for_each(|tag| {
            let layer =
                GeneralMeshLayer::new(ShapePipeline::new(device, global_context,false, style_store.subscribe()));
            layers.shape_layers.insert(tag.clone(), layer);
        });

        layers
    }

    pub fn clear_by_key(&mut self, key: String) {
        self.shape_layers.values().for_each(|layer| {
            // layer..clear_by_key(key.clone());
        });
    }

    pub fn get_layer(&mut self, tag: &String) -> Option<&mut GeneralMeshLayer<ShapePipeline>> {
        self.shape_layers.get_mut(tag)
    }
}

impl BaseMeshLayer for FeatureLayers {
    fn prepare(&mut self, global_context: &mut GlobalContext, device: &Device, config: &SurfaceConfiguration) {
        self.shape_layers.iter_mut().for_each(|(_, layer)| {
            layer.prepare(global_context, device, config);
        });
    }

    fn render(
        &mut self,
        render_pass: &mut RenderPass,
        queue: &Queue,
        device: &Device,
        global_context: &mut GlobalContext,
    ) {
        self.shape_layers.iter_mut().for_each(|(_, layer)| {
            layer.render(render_pass, queue, device, global_context);
        });
    }
}
