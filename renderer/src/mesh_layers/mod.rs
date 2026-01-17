use crate::GlobalContext;
use wgpu::{Device, Queue, RenderPass, SurfaceConfiguration};

pub mod general_mesh_layer;
pub mod text_mesh_layer;

pub trait BaseMeshLayer {
    fn prepare(&mut self, device: &Device, config: &SurfaceConfiguration);
    fn render(
        &mut self,
        render_pass: &mut RenderPass,
        queue: &Queue,
        device: &Device,
        global_context: &mut GlobalContext,
    );
}
