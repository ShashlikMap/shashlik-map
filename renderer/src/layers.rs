use crate::GlobalContext;
use crate::mesh_layers::BaseMeshLayer;
use crate::mesh_layers::general_mesh_layer::GeneralMeshLayer;
use crate::mesh_layers::text_mesh_layer::TextMeshLayer;
use crate::nodes::feature_layers::FeatureLayers;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::shape_pipeline::ShapePipeline;
use crate::pipelines::text_pipeline::TextPipeline;
use crate::styles::style_store::StyleStore;
use rustybuzz::ttf_parser;
use wgpu::{Device, Queue, RenderPass, SurfaceConfiguration};

pub(crate) struct Layers {
    feature_layers: FeatureLayers,
    pub new_shape_layer: GeneralMeshLayer<ShapePipeline>,
    pub new_mesh_layer: GeneralMeshLayer<MeshPipeline>,
    pub new_screen_shape_layer: GeneralMeshLayer<ShapePipeline>,
    pub new_text_layer: TextMeshLayer<TextPipeline>,
}

impl Layers {
    pub fn new(
        device: &Device,
        feature_layers: FeatureLayers,
        style_store: &StyleStore,
        font: &'static ttf_parser::Face<'static>,
    ) -> Layers {
        Layers {
            feature_layers,
            new_mesh_layer: GeneralMeshLayer::new(MeshPipeline::new(device)),
            new_shape_layer: GeneralMeshLayer::new(ShapePipeline::new(
                device,
                false,
                style_store.subscribe(),
            )),
            new_screen_shape_layer: GeneralMeshLayer::new(ShapePipeline::new(
                device,
                true,
                style_store.subscribe(),
            )),
            new_text_layer: TextMeshLayer::new(TextPipeline::new(device), device, font),
        }
    }
    // pub fn shape_layers(&self, index: usize) -> Rc<RefCell<SceneTree>> {
    //     // self.shape_layers.get_shape_layer(index)
    // }

    pub fn feature_layers(&mut self, tag: &String) -> Option<&mut GeneralMeshLayer<ShapePipeline>> {
        self.feature_layers.get_layer(tag)
    }

    pub fn clear(&mut self, key: String) {
        // self.mesh_layer.borrow_mut().clear_by_key(key.clone());
        // self.shape_layers.clear_by_key(key.clone());
        // self.screen_shape_layer
        //     .borrow_mut()
        //     .clear_by_key(key.clone());
        // self.text_layer.borrow_mut().clear_by_key(key.clone());
        self.feature_layers.clear_by_key(key.clone());
    }
}

impl BaseMeshLayer for Layers {
    fn prepare(&mut self, device: &Device, config: &SurfaceConfiguration) {
        self.new_shape_layer.prepare(device, config);
        self.new_mesh_layer.prepare(device, config);
        self.new_screen_shape_layer.prepare(device, config);
        self.new_text_layer.prepare(device, config);
        self.feature_layers.prepare(device, config);
    }

    fn render(
        &mut self,
        render_pass: &mut RenderPass,
        queue: &Queue,
        device: &Device,
        global_context: &mut GlobalContext,
    ) {
        self.new_shape_layer
            .render(render_pass, queue, device, global_context);
        self.new_mesh_layer
            .render(render_pass, queue, device, global_context);
        self.new_screen_shape_layer
            .render(render_pass, queue, device, global_context);
        self.new_text_layer
            .render(render_pass, queue, device, global_context);
        self.feature_layers
            .render(render_pass, queue, device, global_context);
    }
}
