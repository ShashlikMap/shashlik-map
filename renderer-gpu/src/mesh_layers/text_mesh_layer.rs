use crate::global_context::GlobalContext;
use crate::mesh::mesh_instance_input::MeshInstanceInput;
use crate::mesh_layers::{BaseMeshLayer, BaseMeshLayerNew};
use crate::pipelines::RenderPipeline;
use crate::text::text_renderer::TextRenderer;
use renderer_common::geometry_data::{LineData, TextData};
use renderer_common::render_modifier::SpatialData;
use wgpu::RenderPass;

pub struct TextMeshLayer<I: MeshInstanceInput> {
    text_renderer: TextRenderer<I>,
}

impl<I: MeshInstanceInput> TextMeshLayer<I> {
    pub fn new(
        global_context: &mut GlobalContext,
        font: rustybuzz::ttf_parser::Face<'static>,
    ) -> Self {
        Self {
            text_renderer: TextRenderer::new(global_context, font),
        }
    }

    pub fn add(&mut self, key: String, mut text_data: Vec<TextData>, spatial_data: SpatialData) {
        self.text_renderer.update_data(move |holder| {
            text_data.iter_mut().for_each(|item| {
                item.alpha = 0.0;
                item.line_data = LineData::new(item.line_data
                    .positions
                    .iter()
                    .map(|pos| pos + spatial_data.transform)
                    .collect())
            });
            text_data.sort_by(|a, b| {
                let a_len = a.line_data.positions.len();
                let b_len = b.line_data.positions.len();
                a_len.cmp(&b_len)
            });
            holder.set(key.clone(), text_data)
        });
    }

    pub fn run_mut_action_with_key<F>(&mut self, key: &str, block: F)
    where
        F: FnMut(&mut TextData) + Send + 'static,
    {
        let key = key.to_string();
        self.text_renderer
            .update_data(move |holder| holder.run_mut_action_with_key(key.as_str(), block));
    }
}

impl<I: MeshInstanceInput> BaseMeshLayer for TextMeshLayer<I> {

    fn update(&mut self, global_context: &mut GlobalContext) {
        self.text_renderer.update(global_context);
    }

    fn clear_by_key(&mut self, key: &str) {
        let key = key.to_string();
        self.text_renderer.update_data(move |holder| {
            holder.remove(key.as_str());
        });
    }
}

impl <I: MeshInstanceInput> BaseMeshLayerNew<I> for TextMeshLayer<I> {
    fn render_new(&mut self, render_pass: &mut RenderPass, render_pipeline: &mut impl RenderPipeline<I>, global_context: &mut GlobalContext) {
        render_pipeline.setup_render(render_pass, global_context);
        self.text_renderer.render(render_pass, global_context);
    }
}