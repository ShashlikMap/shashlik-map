use crate::global_context::GlobalContext;
use crate::mesh_layers::layers::Layers;
use crate::mesh_layers::BaseMeshComputeLayerNew;
use crate::pass_nodes::PassNode;
use crate::pipelines::shape_pipeline::ShapePipeline;
use renderer_common::WorldShapeFeatureLayerTag;
use wgpu::CommandEncoder;

pub(crate) struct PrepassNode {
    feature_indirect_shape_pipelines: Vec<(String, ShapePipeline)>,
}

impl PrepassNode {
    pub fn new(
        global_context: &GlobalContext,
        world_shape_feature_layer_tag: Vec<WorldShapeFeatureLayerTag>,
    ) -> PrepassNode {

        let feature_indirect_shape_pipelines = world_shape_feature_layer_tag
            .iter()
            .filter(|layer_tag| layer_tag.indirect)
            .map(|tag| {
                let pipeline = ShapePipeline::new(
                    global_context,
                    tag.vertex_shader,
                    true,// force indirect
                    tag.single_instance_step,
                );
                (tag.name.to_string(), pipeline)
            })
            .collect();
        PrepassNode {
            feature_indirect_shape_pipelines,
        }
    }
}

impl PassNode for PrepassNode {
    fn run(
        &mut self,
        encoder: &mut CommandEncoder,
        layers: &mut Layers,
        global_context: &mut GlobalContext,
    ) {
        // only feature layers for now
        self.feature_indirect_shape_pipelines
            .iter_mut()
            .for_each(|(feature_tag, shape_pipeline)| {
                if let Some(layer) = layers.feature_layers.get_layer(feature_tag) {
                    layer.compute_new(encoder, shape_pipeline, global_context)
                }
            });
    }
}
