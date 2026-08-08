pub mod render_to_texture_pass_node;
pub mod main_pass_node;
pub mod prepass_node;
pub mod shadow_pre_pass;
pub mod g_buffer_pass_node;
pub mod ssao_pass_node;

use crate::global_context::GlobalContext;
use crate::mesh_layers::layers::Layers;
use wgpu::{Color, CommandEncoder};

// TODO Ideally, it should be set from Styles somehow
const BACKGROUND_ATTACHMENT_COLOR: Color = Color {
    r: 0.957,
    g: 0.953,
    b: 0.941,
    a: 1.0,
};

pub trait PassNode {
    fn run(
        &self,
        encoder: &mut CommandEncoder,
        layers: &mut Layers,
        global_context: &mut GlobalContext,
    );
}
