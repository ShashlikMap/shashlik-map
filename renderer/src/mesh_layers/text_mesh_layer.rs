use wgpu_canvas::geometry_data::{LineData, TextData};
use crate::global_context::GlobalContext;
use crate::mesh_layers::BaseMeshLayer;
use wgpu_canvas::render_modifier::SpatialData;
use crate::pipelines::RenderPipeline;
use crate::text::text_renderer::TextRenderer;
use wgpu::{CommandEncoder, RenderPass};

pub struct TextMeshLayer<P: RenderPipeline> {
    render_pipeline: P,
    text_renderer: TextRenderer,
    pipeline: Option<wgpu::RenderPipeline>,
}

impl<P: RenderPipeline> TextMeshLayer<P> {
    pub fn new(
        render_pipeline: P,
        global_context: &mut GlobalContext,
        font: &'static rustybuzz::ttf_parser::Face,
    ) -> Self {
        Self {
            render_pipeline,
            text_renderer: TextRenderer::new(global_context, font),
            pipeline: None,
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

impl<P: RenderPipeline> BaseMeshLayer for TextMeshLayer<P> {
    fn prepare(&mut self, global_context: &GlobalContext) {
        let descriptor = self.render_pipeline.prepare(global_context);
        self.pipeline = Some(descriptor.to_render_pipeline(global_context.device()));
    }

    fn update(&mut self, global_context: &mut GlobalContext) {
        if global_context.is_g_buffer_render {
            return;
        }
        self.text_renderer.update(global_context);
    }

    fn compute(&mut self, _encoder: &mut CommandEncoder, _global_context: &mut GlobalContext) {}


    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        if global_context.is_g_buffer_render {
            return;
        }
        if let Some(render_pipeline) = self.pipeline.as_ref() {
            render_pass.set_pipeline(render_pipeline);

            self.render_pipeline.render(render_pass, global_context);
            self.text_renderer.render(render_pass, global_context);
        }
    }

    fn clear_by_key(&mut self, key: &str) {
        let key = key.to_string();
        self.text_renderer.update_data(move |holder| {
            holder.remove(key.as_str());
        });
    }
}
