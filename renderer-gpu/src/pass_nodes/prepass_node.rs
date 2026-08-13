use crate::global_context::GlobalContext;
use crate::mesh_layers::BaseMeshLayer;
use crate::mesh_layers::layers::Layers;
use crate::pass_nodes::PassNode;
use wgpu::CommandEncoder;

pub(crate) struct PrepassNode {}

impl PrepassNode {
    pub fn new() -> PrepassNode {
        PrepassNode {}
    }
}

impl PassNode for PrepassNode {
    fn run(
        &self,
        encoder: &mut CommandEncoder,
        layers: &mut Layers,
        global_context: &mut GlobalContext,
    ) {
        // only feature layers for now
        layers.feature_layers.compute(encoder, global_context);
    }
}
