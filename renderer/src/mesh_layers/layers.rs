use crate::global_context::GlobalContext;
use crate::mesh_layers::feature_layers::{FeatureLayerTag, FeatureLayers};
use crate::mesh_layers::general_mesh_layer::GeneralMeshLayer;
use crate::mesh_layers::ortho_mesh_layer::OrthoMeshLayer;
use crate::mesh_layers::screen_shape_layer::ScreenShapeLayer;
use crate::mesh_layers::text_mesh_layer::TextMeshLayer;
use crate::mesh_layers::BaseMeshLayer;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::screen_mesh_pipeline::ScreenMeshPipeline;
use crate::pipelines::shape_pipeline::ShapePipeline;
use rustybuzz::ttf_parser;
use wgpu::{CommandEncoder, RenderPass};

pub(crate) struct Layers {
    pub is_preview: bool,
    feature_layers: FeatureLayers,
    pub shape_layer: GeneralMeshLayer<ShapePipeline>,
    pub mesh_layer: GeneralMeshLayer<MeshPipeline>,
    pub screen_shape_layer: ScreenShapeLayer<ShapePipeline>,
    pub text_layer: TextMeshLayer<ScreenMeshPipeline>,
    pub ortho_mesh_layer: OrthoMeshLayer<ScreenMeshPipeline>,
}

impl Layers {
    pub fn new(
        feature_tags: Vec<FeatureLayerTag>,
        global_context: &mut GlobalContext,
        font: &'static ttf_parser::Face<'static>,
    ) -> Layers {
        let feature_layers = FeatureLayers::new(feature_tags, global_context);
        Layers {
            is_preview: false,
            feature_layers,
            mesh_layer: GeneralMeshLayer::new(MeshPipeline::new(global_context)),
            shape_layer: GeneralMeshLayer::new(ShapePipeline::new(global_context, None, false)),
            screen_shape_layer: ScreenShapeLayer::new(ShapePipeline::new(global_context, Some("vs_main_screen"), false),
                                                      global_context),
            text_layer: TextMeshLayer::new(
                ScreenMeshPipeline::new(global_context, false),
                global_context,
                font,
            ),
            ortho_mesh_layer: OrthoMeshLayer::new(ScreenMeshPipeline::new(global_context, true)),
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
        self.ortho_mesh_layer.prepare(global_context);
    }

    fn update(&mut self, global_context: &mut GlobalContext) {
        self.shape_layer.update(global_context);
        self.mesh_layer.update(global_context);
        self.screen_shape_layer.update(global_context);
        self.text_layer.update(global_context);
        self.feature_layers.update(global_context);
        self.ortho_mesh_layer.update(global_context);
    }

    fn compute(&mut self, encoder: &mut CommandEncoder, global_context: &mut GlobalContext) {
        // only feature layer for now
        self.feature_layers.compute(encoder, global_context);
    }


    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        self.shape_layer.disable_skip_mesh_feature = self.is_preview;
        self.shape_layer.render(render_pass, global_context);
        if !self.is_preview {
            self.mesh_layer.render(render_pass, global_context);
            self.screen_shape_layer
                .render(render_pass, global_context);
            self.text_layer.render(render_pass, global_context);
        }
        self.feature_layers.render(render_pass, global_context);
        if !self.is_preview {
            self.ortho_mesh_layer.render(render_pass, global_context);
        }
    }

    fn clear_by_key(&mut self, key: &str) {
        self.shape_layer.clear_by_key(key);
        self.mesh_layer.clear_by_key(key);
        self.screen_shape_layer.clear_by_key(key);
        self.text_layer.clear_by_key(key);
        self.feature_layers.clear_by_key(key);
        self.ortho_mesh_layer.clear_by_key(key);
    }
}
