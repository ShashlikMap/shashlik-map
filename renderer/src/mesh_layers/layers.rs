use crate::global_context::GlobalContext;
use crate::mesh_layers::BaseMeshLayer;
use crate::mesh_layers::feature_layers::FeatureLayers;
use crate::mesh_layers::general_mesh_layer::GeneralMeshLayer;
use crate::mesh_layers::text_mesh_layer::TextMeshLayer;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::shape_pipeline::ShapePipeline;
use crate::pipelines::text_pipeline::TextPipeline;
use rustybuzz::ttf_parser;
use wgpu::RenderPass;
use crate::mesh_layers::screen_mesh_layer::ScreenMeshLayer;

pub(crate) struct Layers {
    pub is_preview: bool,
    feature_layers: FeatureLayers,
    pub shape_layer: GeneralMeshLayer<ShapePipeline>,
    pub mesh_layer: GeneralMeshLayer<MeshPipeline>,
    pub screen_shape_layer: ScreenMeshLayer<ShapePipeline>,
    pub text_layer: TextMeshLayer<TextPipeline>,
}

impl Layers {
    pub fn new(
        feature_tags: &[String],
        global_context: &mut GlobalContext,
        font: &'static ttf_parser::Face<'static>,
    ) -> Layers {
        let feature_layers = FeatureLayers::new(feature_tags, global_context);
        Layers {
            is_preview: false,
            feature_layers,
            mesh_layer: GeneralMeshLayer::new(MeshPipeline::new(global_context)),
            shape_layer: GeneralMeshLayer::new(ShapePipeline::new(global_context, false)),
            screen_shape_layer: ScreenMeshLayer::new(ShapePipeline::new(global_context, true),
                                                     global_context),
            text_layer: TextMeshLayer::new(
                TextPipeline::new(global_context),
                global_context,
                font,
            ),
        }
    }

    pub fn feature_layers(&mut self, tag: &String) -> Option<&mut GeneralMeshLayer<ShapePipeline>> {
        self.feature_layers.get_layer(tag)
    }
}

// TODO Refactor
impl BaseMeshLayer for Layers {
    fn prepare(&mut self, global_context: &GlobalContext) {
        self.shape_layer.prepare(global_context);
        self.mesh_layer.prepare(global_context);
        self.screen_shape_layer.prepare(global_context);
        self.text_layer.prepare(global_context);
        self.feature_layers.prepare(global_context);
    }

    fn update(&mut self, global_context: &mut GlobalContext) {
        self.shape_layer.update(global_context);
        self.mesh_layer.update(global_context);
        self.screen_shape_layer.update(global_context);
        self.text_layer.update(global_context);
        self.feature_layers.update(global_context);
    }

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        self.shape_layer.render(render_pass, global_context);
        if !self.is_preview {
            self.mesh_layer.render(render_pass, global_context);
            self.screen_shape_layer
                .render(render_pass, global_context);
            self.text_layer.render(render_pass, global_context);
        }
        self.feature_layers.render(render_pass, global_context);
    }

    fn clear_by_key(&mut self, key: &str) {
        self.shape_layer.clear_by_key(key);
        self.mesh_layer.clear_by_key(key);
        self.screen_shape_layer.clear_by_key(key);
        self.text_layer.clear_by_key(key);
        self.feature_layers.clear_by_key(key);
    }
}
