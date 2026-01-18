use crate::GlobalContext;
use crate::geometry_data::TextData;
use crate::mesh_layers::BaseMeshLayer;
use crate::mesh_layers::render_data_holder::RenderDataHolder;
use crate::modifier::render_modifier::SpatialData;
use crate::pipelines::RenderPipeline;
use crate::text::text_renderer::TextRenderer;
use wgpu::RenderPass;

pub struct TextMeshLayer<P: RenderPipeline> {
    render_pipeline: P,
    render_data_holder: RenderDataHolder<Vec<TextData>>,
    pub(crate) text_renderer: TextRenderer,
    pipeline: Option<wgpu::RenderPipeline>,
}

impl<P: RenderPipeline> TextMeshLayer<P> {
    pub fn new(
        render_pipeline: P,
        global_context: &GlobalContext,
        font: &'static rustybuzz::ttf_parser::Face,
    ) -> Self {
        Self {
            render_pipeline,
            render_data_holder: RenderDataHolder::new(),
            text_renderer: TextRenderer::new(global_context.device(), font),
            pipeline: None,
        }
    }

    pub fn add(&mut self, key: String, mut text_data: Vec<TextData>, spatial_data: SpatialData) {
        text_data.iter_mut().for_each(|item| {
            item.alpha = 0.0;
            item.positions = item
                .positions
                .iter()
                .map(|pos| pos + spatial_data.transform.cast().unwrap())
                .collect()
        });

        self.render_data_holder.add(key, text_data);
    }
}

impl<P: RenderPipeline> BaseMeshLayer for TextMeshLayer<P> {
    fn prepare(&mut self, global_context: &GlobalContext) {
        let descriptor = self.render_pipeline.prepare(global_context);
        self.pipeline = Some(descriptor.to_render_pipeline(global_context.device()));
    }

    fn update(&mut self, global_context: &mut GlobalContext) {
        self.render_data_holder
            .holder
            .iter_mut()
            .for_each(|(_, data)| {
                data.iter_mut().for_each(|items| {
                    items
                        .iter_mut()
                        .for_each(|item| self.text_renderer.insert(item, global_context));
                });
            });

    }


    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        // self.render_data_holder
        //     .holder
        //     .iter_mut()
        //     .for_each(|(_, data)| {
        //         data.iter_mut()
        //             .for_each(|item| self.text_renderer.insert(item, global_context));
        //     });
        //
        // self.text_renderer.update(&global_context);

        if let Some(pp) = self.pipeline.as_ref() {
            render_pass.set_pipeline(pp);

            self.render_pipeline.render(render_pass, global_context);
            self.text_renderer.render(render_pass, global_context);
        }
    }

    fn clear_by_key(&mut self, key: String) {
        self.render_data_holder.remove(key);
    }
}
