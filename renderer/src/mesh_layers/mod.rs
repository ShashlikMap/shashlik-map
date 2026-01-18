use crate::GlobalContext;
use wgpu::RenderPass;

pub mod feature_layers;
pub mod general_mesh_layer;
pub mod text_mesh_layer;

pub trait BaseMeshLayer {
    fn prepare(&mut self, global_context: &GlobalContext);
    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext);
}
