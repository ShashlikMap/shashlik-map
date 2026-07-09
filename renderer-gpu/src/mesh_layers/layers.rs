use crate::global_context::GlobalContext;
use crate::mesh_layers::feature_layers::{FeatureLayerTag, FeatureLayers, NameLayerTag};
use crate::mesh_layers::general_mesh_layer::GeneralMeshLayer;
use crate::mesh_layers::ortho_mesh_layer::OrthoMeshLayer;
use crate::mesh_layers::screen_shape_layer::ScreenShapeLayer;
use crate::mesh_layers::text_mesh_layer::TextMeshLayer;
use crate::mesh_layers::BaseMeshLayer;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::screen_mesh_pipeline::{ScreenMeshPipeline, TextureInfo};
use crate::pipelines::shape_pipeline::ShapePipeline;
use rustybuzz::ttf_parser;
use wgpu::{CommandEncoder, RenderPass};
use renderer_common::{WorldShapeFeatureLayerTag, PREVIEW_TYPE, SSAO_ENABLED};

pub(crate) const WORLD_TEXT_LAYER: &'static str = "world_text_layer";
pub(crate) const SCREEN_TEXT_LAYER: &'static str = "screen_text_layer";


impl FeatureLayerTag for WorldShapeFeatureLayerTag {
    fn name(&self) -> &'static str {
        self.name
    }
}

pub(crate) struct Layers {
    feature_layers: FeatureLayers<GeneralMeshLayer<ShapePipeline>>,
    pub shape_layer: GeneralMeshLayer<ShapePipeline>,
    pub mesh_layer: GeneralMeshLayer<MeshPipeline>,
    pub shadow_map_layer: OrthoMeshLayer<ScreenMeshPipeline>,
    pub screen_shape_layer: ScreenShapeLayer<ShapePipeline>,
    pub text_feature_layers: FeatureLayers<TextMeshLayer<ScreenMeshPipeline>>,
    pub preview_mesh_layer: OrthoMeshLayer<ScreenMeshPipeline>,
    pub post_process_layer: OrthoMeshLayer<ScreenMeshPipeline>,
}

impl Layers {
    pub fn new(
        world_shapes_feature_tags: Vec<WorldShapeFeatureLayerTag>,
        global_context: &mut GlobalContext,
        font: &'static ttf_parser::Face<'static>,
    ) -> Layers {
        let feature_layers = FeatureLayers::new(world_shapes_feature_tags,
                                                |tag| {
                                                    GeneralMeshLayer::new(ShapePipeline::new(global_context, tag.vertex_shader, tag.indirect),
                                                    false)
                                                });
        let text_feature_layers = FeatureLayers::new(vec![
            NameLayerTag(WORLD_TEXT_LAYER),
            NameLayerTag(SCREEN_TEXT_LAYER)], |_| {
            TextMeshLayer::new(
                ScreenMeshPipeline::new(global_context, TextureInfo {
                    use_texture: false,
                    filterable: false,
                    vs_shader: None,
                    fs_shader: "",
                }),
                global_context,
                font,
            )
        });

        Layers {
            feature_layers,
            mesh_layer: GeneralMeshLayer::new(MeshPipeline::new(global_context), true),
            shape_layer: GeneralMeshLayer::new(ShapePipeline::new(global_context, None, false), false),
            screen_shape_layer: ScreenShapeLayer::new(ShapePipeline::new(global_context, Some("vs_main_screen"), false),
                                                      global_context),
            shadow_map_layer: OrthoMeshLayer::new(ScreenMeshPipeline::new(global_context, TextureInfo {
                use_texture: true,
                filterable: false,
                vs_shader: Some("vs_main_sm"),
                fs_shader: "fs_main_sm",
            }), true, false, true),
            text_feature_layers,
            preview_mesh_layer: OrthoMeshLayer::new(ScreenMeshPipeline::new(global_context, TextureInfo {
                use_texture: true,
                filterable: true,
                vs_shader: None,
                fs_shader: "fs_main_textured",
            }), false, true, false),
            post_process_layer: OrthoMeshLayer::new(ScreenMeshPipeline::new(global_context, TextureInfo {
                use_texture: true,
                filterable: true,
                vs_shader: None,
                fs_shader: "fs_main_tex_storage",
            }), true, false, false),
        }
    }

    pub fn feature_layers(&mut self, tag: &str) -> Option<&mut GeneralMeshLayer<ShapePipeline>> {
        self.feature_layers.get_layer(tag)
    }
}

// TODO Refactor
impl BaseMeshLayer for Layers {
    fn prepare(&mut self, global_context: &GlobalContext) {
        self.shape_layer.prepare(global_context);
        self.mesh_layer.prepare(global_context);
        self.shadow_map_layer.prepare(global_context);
        self.screen_shape_layer.prepare(global_context);
        self.text_feature_layers.prepare(global_context);
        self.feature_layers.prepare(global_context);
        self.preview_mesh_layer.prepare(global_context);
        self.post_process_layer.prepare(global_context);
    }

    fn update(&mut self, global_context: &mut GlobalContext) {
        self.shape_layer.update(global_context);
        self.mesh_layer.update(global_context);
        self.shadow_map_layer.update(global_context);
        self.screen_shape_layer.update(global_context);
        self.text_feature_layers.update(global_context);
        self.feature_layers.update(global_context);
        self.preview_mesh_layer.update(global_context);
        self.post_process_layer.update(global_context);
    }

    fn compute(&mut self, encoder: &mut CommandEncoder, global_context: &mut GlobalContext) {
        // only feature layer for now
        self.feature_layers.compute(encoder, global_context);
    }

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        if !global_context.is_g_buffer_render && !global_context.is_shadow_render {
            self.shape_layer.disable_skip_mesh_feature = global_context.is_preview_render;
            self.shape_layer.render(render_pass, global_context);
        }
        if !global_context.is_preview_render {
            self.mesh_layer.render(render_pass, global_context);

            let not_shadow_or_g_buf = !global_context.is_g_buffer_render && !global_context.is_shadow_render;
            if global_context.is_shadow_mapping_enabled() && not_shadow_or_g_buf {
                self.shadow_map_layer.render(render_pass, global_context);
            }

            if unsafe { SSAO_ENABLED } && not_shadow_or_g_buf {
                self.post_process_layer.render(render_pass, global_context);
            }
            if not_shadow_or_g_buf {
                self.screen_shape_layer
                    .render(render_pass, global_context);
                self.text_feature_layers.render(render_pass, global_context);
            }
        }
        if !global_context.is_g_buffer_render && !global_context.is_shadow_render {
            self.feature_layers.render(render_pass, global_context);
        }
        if unsafe { PREVIEW_TYPE.is_enabled() } && !global_context.is_preview_render && !global_context.is_shadow_render {
            self.preview_mesh_layer.render(render_pass, global_context);
        }
    }

    fn clear_by_key(&mut self, key: &str) {
        self.shape_layer.clear_by_key(key);
        self.mesh_layer.clear_by_key(key);
        self.shadow_map_layer.clear_by_key(key);
        self.screen_shape_layer.clear_by_key(key);
        self.text_feature_layers.clear_by_key(key);
        self.feature_layers.clear_by_key(key);
        self.preview_mesh_layer.clear_by_key(key);
        self.post_process_layer.clear_by_key(key);
    }
}
