pub mod render_to_texture_pass_node;
pub mod main_pass_node;
pub mod prepass_node;
pub mod shadow_pre_pass;

use crate::global_context::GlobalContext;
use crate::mesh_layers::layers::Layers;
use wgpu::{CommandEncoder, TextureView};

pub trait PassNode {

    fn compute(
        &mut self,
        encoder: &mut CommandEncoder,
        layers: &mut Layers,
        global_context: &mut GlobalContext,
    );

    fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        output_view: &TextureView,
        layers: &mut Layers,
        global_context: &mut GlobalContext,
    );
}
