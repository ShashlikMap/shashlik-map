use crate::global_context::GlobalContext;
use wgpu::{CommandEncoder, RenderPass};
use crate::mesh::mesh_instance_input::MeshInstanceInput;
use crate::pipelines::RenderPipeline;

pub mod general_mesh_layer;
pub mod text_mesh_layer;
pub mod render_data_holder;
pub mod layers;
pub mod screen_shape_layer;
pub mod ortho_mesh_layer;
pub mod feature_layers;

#[derive(Default)]
pub struct LayerAttribute {
    pub(crate) position: [f32; 3],
    pub(crate) color_alpha: f32,
    pub(crate) matrix: [[f32; 4]; 4],
    pub(crate) bbox: [f32; 4],
    pub(crate) normal_scale: f32,
    pub(crate) screen_space: u32,
}

pub(crate) type LayerAttrMapper<I> = fn(LayerAttribute) -> I;

pub(crate) trait BaseMeshLayer {
    fn update(&mut self, global_context: &mut GlobalContext);

    fn clear_by_key(&mut self, key: &str);
}

pub(crate) trait RenderableLayer<I: MeshInstanceInput> {
    fn render(&mut self, _render_pass: &mut RenderPass, _render_pipeline: &mut impl RenderPipeline<I>, _global_context: &mut GlobalContext) {}
}

pub(crate) trait ComputableLayer<I: MeshInstanceInput> {
    fn compute(&mut self, _command_encoder: &mut CommandEncoder, _render_pipeline: &mut impl RenderPipeline<I>, _global_context: &mut GlobalContext) {}
}
