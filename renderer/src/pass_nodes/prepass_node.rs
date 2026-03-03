use crate::global_context::GlobalContext;
use crate::mesh_layers::layers::Layers;
use crate::mesh_layers::BaseMeshLayer;
use crate::pass_nodes::PassNode;
use wgpu::{CommandEncoder, TextureView};

pub(crate) struct PrepassNode {}

impl PrepassNode {
    pub fn new() -> PrepassNode {
        PrepassNode {}
    }
}

impl PassNode for PrepassNode {
    fn compute(
        &mut self,
        encoder: &mut CommandEncoder,
        layers: &mut Layers,
        global_context: &mut GlobalContext,
    ) {
        layers.compute(encoder, global_context);
    }

    fn render(
        &mut self,
        _encoder: &mut CommandEncoder,
        _output_view: &TextureView,
        _layers: &mut Layers,
        _global_context: &mut GlobalContext,
    ) {
        // no special rendering for prepass
    }
}
