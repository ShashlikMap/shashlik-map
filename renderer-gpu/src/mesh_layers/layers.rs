use crate::global_context::GlobalContext;
use crate::mesh_layers::BaseMeshLayer;
use crate::mesh_layers::feature_layers::{FeatureLayerTag, FeatureLayers, NameLayerTag};
use crate::mesh_layers::general_mesh_layer::GeneralMeshLayer;
use crate::mesh_layers::ortho_mesh_layer::OrthoMeshLayer;
use crate::mesh_layers::screen_shape_layer::ScreenShapeLayer;
use crate::mesh_layers::text_mesh_layer::TextMeshLayer;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::screen_mesh_pipeline::{ScreenMeshPipeline, TextureInfo};
use crate::pipelines::shape_pipeline::ShapePipeline;
use renderer_common::WorldShapeFeatureLayerTag;
use rustybuzz::ttf_parser;

pub(crate) const WORLD_TEXT_LAYER: &'static str = "world_text_layer";
pub(crate) const SCREEN_TEXT_LAYER: &'static str = "screen_text_layer";

impl FeatureLayerTag for WorldShapeFeatureLayerTag {
    fn name(&self) -> &'static str {
        self.name
    }
}

pub(crate) struct Layers {
    pub world_shapes_feature_tags: Vec<WorldShapeFeatureLayerTag>,
    pub feature_layers: FeatureLayers<GeneralMeshLayer<ShapePipeline>>,
    pub shape_layer: GeneralMeshLayer<ShapePipeline>,
    pub mesh_layer: GeneralMeshLayer<MeshPipeline>,
    pub shadow_map_layer: OrthoMeshLayer<ScreenMeshPipeline>,
    pub screen_shape_layer: ScreenShapeLayer<ShapePipeline>,
    pub text_feature_layers: FeatureLayers<TextMeshLayer<ScreenMeshPipeline>>,
    pub preview_mesh_layer: OrthoMeshLayer<ScreenMeshPipeline>,
    pub post_process_layer: OrthoMeshLayer<ScreenMeshPipeline>,
}

impl Layers {
    pub fn text_layer_tags() -> Vec<NameLayerTag> {
        vec![
            NameLayerTag(WORLD_TEXT_LAYER),
            NameLayerTag(SCREEN_TEXT_LAYER),
        ]
    }

    pub fn new(
        world_shapes_feature_tags: Vec<WorldShapeFeatureLayerTag>,
        global_context: &mut GlobalContext,
        font: ttf_parser::Face<'static>,
    ) -> Layers {
        let feature_layers = FeatureLayers::new(world_shapes_feature_tags.clone(), |tag| {
            GeneralMeshLayer::new(
                ShapePipeline::new(
                    global_context,
                    tag.vertex_shader,
                    tag.indirect,
                    tag.single_instance_step,
                ),
            )
        });
        let text_feature_layers = FeatureLayers::new(
            vec![
                NameLayerTag(WORLD_TEXT_LAYER),
                NameLayerTag(SCREEN_TEXT_LAYER),
            ],
            |_| {
                TextMeshLayer::new(
                    ScreenMeshPipeline::new(
                        global_context,
                        TextureInfo {
                            use_texture: false,
                            filterable: false,
                            vs_shader: None,
                            fs_shader: "",
                        },
                        false
                    ),
                    global_context,
                    font.clone(),
                )
            },
        );

        Layers {
            world_shapes_feature_tags,
            feature_layers,
            mesh_layer: GeneralMeshLayer::new(MeshPipeline::new(global_context, true, true, false)),
            shape_layer: GeneralMeshLayer::new(
                ShapePipeline::new(global_context, None, false, true),
            ),
            screen_shape_layer: ScreenShapeLayer::new(
                ShapePipeline::new(global_context, Some("vs_main_screen"), false, false),
                global_context,
            ),
            shadow_map_layer: OrthoMeshLayer::new(
                ScreenMeshPipeline::new(
                    global_context,
                    TextureInfo {
                        use_texture: true,
                        filterable: false,
                        vs_shader: Some("vs_main_sm"),
                        fs_shader: "fs_main_sm",
                    },
                    true
                ),
                true,
                false,
            ),
            text_feature_layers,
            preview_mesh_layer: OrthoMeshLayer::new(
                ScreenMeshPipeline::new(
                    global_context,
                    TextureInfo {
                        use_texture: true,
                        filterable: true,
                        vs_shader: None,
                        fs_shader: "fs_main_textured",
                    },
                    false
                ),
                false,
                true,
            ),
            post_process_layer: OrthoMeshLayer::new(
                ScreenMeshPipeline::new(
                    global_context,
                    TextureInfo {
                        use_texture: true,
                        filterable: true,
                        vs_shader: None,
                        fs_shader: "fs_main_tex_storage",
                    },
                    false
                ),
                true,
                false,
            ),
        }
    }

    pub fn prepare(&mut self, global_context: &GlobalContext) {
        self.all_layers()
            .iter_mut()
            .for_each(|layer| layer.prepare(global_context));
    }

    pub fn update(&mut self, global_context: &mut GlobalContext) {
        self.all_layers()
            .iter_mut()
            .for_each(|layer| layer.update(global_context));
    }

    pub fn clear_by_key(&mut self, key: &str) {
        self.all_layers()
            .iter_mut()
            .for_each(|layer| layer.clear_by_key(key));
    }

    pub fn feature_layers(&mut self, tag: &str) -> Option<&mut GeneralMeshLayer<ShapePipeline>> {
        self.feature_layers.get_layer(tag)
    }

    fn all_layers(&mut self) -> [&mut dyn BaseMeshLayer; 8] {
        [
            &mut self.shape_layer,
            &mut self.mesh_layer,
            &mut self.shadow_map_layer,
            &mut self.post_process_layer,
            &mut self.screen_shape_layer,
            &mut self.text_feature_layers,
            &mut self.feature_layers,
            &mut self.preview_mesh_layer,
        ]
    }
}