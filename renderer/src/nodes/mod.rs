use crate::GlobalContext;
use wgpu::RenderPass;

pub mod mesh_node;

pub trait SceneNode {
    fn update(
        &mut self,
        _global_context: &mut GlobalContext,
    ) {
    }
    fn render(&mut self, _render_pass: &mut RenderPass, _global_context: &mut GlobalContext) {}
}