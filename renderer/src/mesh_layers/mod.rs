use crate::global_context::GlobalContext;
use wgpu::RenderPass;

pub mod feature_layers;
pub mod general_mesh_layer;
pub mod text_mesh_layer;
pub mod render_data_holder;
pub mod layers;
pub mod screen_mesh_layer;
pub mod ortho_mesh_layer;

pub trait BaseMeshLayer {
    fn prepare(&mut self, global_context: &GlobalContext);

    fn update(&mut self, global_context: &mut GlobalContext);
    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext);

    fn clear_by_key(&mut self, key: &str);
}
