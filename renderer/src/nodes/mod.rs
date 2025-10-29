use crate::nodes::scene_tree::RenderContext;
use wgpu::{Device, Queue, RenderPass};

pub mod world;
pub mod camera_node;
pub(crate) mod mesh_node;
pub mod mesh_layer;
pub mod scene_tree;
pub mod fps_node;
pub mod style_adapter_node;

pub trait SceneNode {
    fn setup(&mut self, _render_context: &mut RenderContext, _device: &Device) {}
    fn update(&mut self, _device: &Device, _queue: &Queue) {}
    fn render(&self, _render_pass: &mut RenderPass) {}
    fn resize(&mut self, _width: u32, _height: u32, _queue: &Queue) {}
}
