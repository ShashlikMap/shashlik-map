use crate::global_context::GlobalContext;
use crate::mesh_layers::BaseMeshLayer;
use crate::mesh_layers::feature_layers::{FeatureLayerTag, FeatureLayers, NameLayerTag};
use crate::mesh_layers::general_mesh_layer::GeneralMeshLayer;
use crate::mesh_layers::ortho_mesh_layer::OrthoMeshLayer;
use crate::mesh_layers::screen_shape_layer::ScreenShapeLayer;
use crate::mesh_layers::text_mesh_layer::TextMeshLayer;
use crate::vertex_attrs::{GeneralInstanceInput, ShapeInstanceInput, ScreenShapeInstanceInput};
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
    pub feature_layers: FeatureLayers<GeneralMeshLayer<ShapeInstanceInput>>,
    pub shape_layer: GeneralMeshLayer<ShapeInstanceInput>,
    pub mesh_layer: GeneralMeshLayer<GeneralInstanceInput>,
    pub shadow_map_layer: OrthoMeshLayer<ScreenShapeInstanceInput>,
    pub screen_shape_layer: ScreenShapeLayer<ShapeInstanceInput>,
    pub text_feature_layers: FeatureLayers<TextMeshLayer<ScreenShapeInstanceInput>>,
    pub preview_mesh_layer: OrthoMeshLayer<ScreenShapeInstanceInput>,
    pub post_process_layer: OrthoMeshLayer<ScreenShapeInstanceInput>,
}

impl Layers {
    pub fn new(
        world_shapes_feature_tags: Vec<WorldShapeFeatureLayerTag>,
        global_context: &mut GlobalContext,
        font: ttf_parser::Face<'static>,
    ) -> Layers {
        let feature_layers = FeatureLayers::new(world_shapes_feature_tags.clone(), |tag| {
            GeneralMeshLayer::new(
                tag.indirect
            )
        });
        let text_feature_layers = FeatureLayers::new(
            vec![
                NameLayerTag(WORLD_TEXT_LAYER),
                NameLayerTag(SCREEN_TEXT_LAYER),
            ],
            |_| {
                TextMeshLayer::new(
                    global_context,
                    font.clone(),
                )
            },
        );

        Layers {
            world_shapes_feature_tags,
            feature_layers,
            mesh_layer: GeneralMeshLayer::new(false),
            shape_layer: GeneralMeshLayer::new(false),
            screen_shape_layer: ScreenShapeLayer::new(global_context),
            shadow_map_layer: OrthoMeshLayer::new(true, false),
            text_feature_layers,
            preview_mesh_layer: OrthoMeshLayer::new(false, true),
            post_process_layer: OrthoMeshLayer::new(true, false),
        }
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

    pub fn feature_layers(&mut self, tag: &str) -> Option<&mut GeneralMeshLayer<ShapeInstanceInput>> {
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