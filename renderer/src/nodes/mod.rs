use crate::nodes::scene_tree::RenderContext;
use crate::GlobalContext;
use wgpu::{Device, Queue, RenderPass};

pub mod camera_node;
pub mod fps_node;
pub mod mesh_layer;
pub(crate) mod mesh_node;
pub mod scene_tree;
pub mod style_adapter_node;
pub mod text_node;
pub mod world;

pub trait SceneNode {
    fn setup(&mut self, _render_context: &mut RenderContext, _device: &Device) {}
    fn update(
        &mut self,
        _device: &Device,
        _queue: &Queue,
        _config: &wgpu::SurfaceConfiguration,
        _global_context: &mut GlobalContext,
    ) {
    }
    fn render(&self, _render_pass: &mut RenderPass, _global_context: &mut GlobalContext) {}
    fn resize(&mut self, _width: u32, _height: u32, _queue: &Queue) {}
}
