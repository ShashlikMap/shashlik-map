use wgpu::{Device, Queue, RenderPass};
use crate::GlobalContext;

pub mod feature_layers;
pub mod mesh_node;

pub trait SceneNode {
    fn update(
        &mut self,
        _device: &Device,
        _queue: &Queue,
        _global_context: &mut GlobalContext,
    ) {
    }
    fn render(&mut self, _render_pass: &mut RenderPass, _global_context: &mut GlobalContext) {}
    fn resize(&mut self, _width: u32, _height: u32, _queue: &Queue) {}
}