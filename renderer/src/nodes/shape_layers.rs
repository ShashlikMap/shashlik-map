use crate::nodes::mesh_layer::MeshLayer;
use crate::nodes::scene_tree::SceneTree;
use crate::nodes::style_adapter_node::StyleAdapterNode;
use crate::pipeline_provider::PipeLineProvider;
use crate::styles::style_store::StyleStore;
use crate::vertex_attrs::{InstancePos, ShapeVertex, VertexAttrib};
use crate::SHADER_STYLE_GROUP_INDEX;
use std::cell::RefCell;
use std::cmp::min;
use std::rc::Rc;
use wgpu::{include_wgsl, CompareFunction, Device};

const MAX_SHAPE_LAYERS: usize = 2;
pub struct ShapeLayers {
    shape_layers: Vec<Rc<RefCell<SceneTree>>>,
}

impl ShapeLayers {
    pub fn new(
        device: &Device,
        pipeline_provider: PipeLineProvider,
        style_store: &StyleStore,
        camera_node: Rc<RefCell<SceneTree>>,
    ) -> ShapeLayers {
        let mut shape_layers = Vec::with_capacity(MAX_SHAPE_LAYERS);
        for i in 0..MAX_SHAPE_LAYERS {
            let shape_layer = MeshLayer::new(
                &device,
                include_wgsl!("../shaders/shape_shader.wgsl"),
                Rc::new([ShapeVertex::desc(), InstancePos::desc()]),
                pipeline_provider.clone(),
                None,
                CompareFunction::Less
            );

            let shape_layer: StyleAdapterNode<MeshLayer> = StyleAdapterNode::new(
                device,
                style_store.subscribe(),
                shape_layer,
                SHADER_STYLE_GROUP_INDEX,
                CompareFunction::Always,
            );

            let shape_layer = camera_node
                .borrow_mut()
                .add_child_with_key(shape_layer, format!("shape_layer {i}").to_string());
            shape_layers.push(shape_layer);
        }

        ShapeLayers { shape_layers }
    }

    pub fn clear_by_key(&mut self, key: String) {
        self.shape_layers.iter().for_each(|layer| {
            layer.borrow_mut().clear_by_key(key.clone());
        });
    }

    pub fn get_shape_layer(&self, index: usize) -> Rc<RefCell<SceneTree>> {
        self.shape_layers[min(index, self.shape_layers.len() - 1)].clone()
    }
}
