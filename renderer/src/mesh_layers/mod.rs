use crate::GlobalContext;
use wgpu::{Device, Queue, RenderPass, SurfaceConfiguration};

pub mod general_mesh_layer;
pub mod text_mesh_layer;
pub mod feature_layers;

pub trait BaseMeshLayer {
    fn prepare(&mut self, global_context: &mut GlobalContext, device: &Device, config: &SurfaceConfiguration);
    fn render(
        &mut self,
        render_pass: &mut RenderPass,
        queue: &Queue,
        device: &Device,
        global_context: &mut GlobalContext,
    );
}
