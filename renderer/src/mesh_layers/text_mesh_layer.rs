use crate::GlobalContext;
use crate::geometry_data::TextData;
use crate::mesh_layers::BaseMeshLayer;
use crate::modifier::render_modifier::SpatialData;
use crate::pipelines::RenderPipeline;
use crate::text::text_renderer::TextRenderer;
use wgpu::{Device, Queue, RenderPass, SurfaceConfiguration};

pub struct TextMeshLayer<P: RenderPipeline> {
    render_pipeline: P,
    meshes: Vec<Vec<TextData>>,
    text_renderer: TextRenderer,
    pipeline: Option<wgpu::RenderPipeline>,
}

impl<P: RenderPipeline> TextMeshLayer<P> {
    pub fn new(
        render_pipeline: P,
        device: &Device,
        font: &'static rustybuzz::ttf_parser::Face,
    ) -> Self {
        Self {
            render_pipeline,
            meshes: vec![],
            text_renderer: TextRenderer::new(device, font),
            pipeline: None,
        }
    }

    pub fn add(&mut self, mut text_data: Vec<TextData>, spatial_data: SpatialData) {
        text_data.iter_mut().for_each(|item| {
            item.alpha = 0.0;
            item.positions = item
                .positions
                .iter()
                .map(|pos| pos + spatial_data.transform.cast().unwrap())
                .collect()
        });

        self.meshes.push(text_data);
    }
}

impl<P: RenderPipeline> BaseMeshLayer for TextMeshLayer<P> {
    fn prepare(&mut self, device: &Device, config: &SurfaceConfiguration) {
        let descriptor = self.render_pipeline.prepare(device, config);
        self.pipeline = Some(descriptor.to_render_pipeline(device));
    }

    fn render(
        &mut self,
        render_pass: &mut RenderPass,
        queue: &Queue,
        device: &Device,
        global_context: &mut GlobalContext,
    ) {
        self.meshes.iter_mut().for_each(|data| {
            data.iter_mut().for_each(|item| {
                self.text_renderer.insert(
                    item,
                    &mut global_context.collision_handler,
                    &global_context.view_projection,
                )
            });
        });

        self.text_renderer
            .update(queue, device, &global_context.view_projection.cs_offset);

        if let Some(pp) = self.pipeline.as_ref() {
            render_pass.set_pipeline(pp);

            self.render_pipeline
                .render(render_pass, device, queue, global_context);
            self.text_renderer.render(render_pass);
        }
    }
}
