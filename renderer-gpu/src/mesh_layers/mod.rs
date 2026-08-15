use crate::global_context::GlobalContext;
use wgpu::{CommandEncoder, RenderPass};
use crate::pipelines::RenderPipeline;

pub mod general_mesh_layer;
pub mod text_mesh_layer;
pub mod render_data_holder;
pub mod layers;
pub mod screen_shape_layer;
pub mod ortho_mesh_layer;
pub mod feature_layers;

pub trait BaseMeshLayer {
    fn update(&mut self, global_context: &mut GlobalContext);

    fn clear_by_key(&mut self, key: &str);
}

pub trait BaseMeshLayerNew {
    fn render_new(&mut self, _render_pass: &mut RenderPass, _render_pipeline: &mut impl RenderPipeline, _global_context: &mut GlobalContext) {}
}

pub trait BaseMeshComputeLayerNew {
    fn compute_new(&mut self, _command_encoder: &mut CommandEncoder, _render_pipeline: &mut impl RenderPipeline, _global_context: &mut GlobalContext) {}
}
