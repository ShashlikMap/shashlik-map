use crate::SHADER_STYLE_GROUP_INDEX;
use crate::nodes::mesh_layer::MeshLayer;
use crate::nodes::scene_tree::SceneTree;
use crate::nodes::style_adapter_node::StyleAdapterNode;
use crate::pipeline_provider::PipeLineProvider;
use crate::styles::style_store::StyleStore;
use crate::vertex_attrs::{InstancePos, ShapeVertex, VertexAttrib};
use linked_hash_map::LinkedHashMap;
use std::cell::RefCell;
use std::rc::Rc;
use wgpu::{CompareFunction, Device, include_wgsl};

pub struct FeatureLayers {
    shape_layers: LinkedHashMap<String, Rc<RefCell<SceneTree>>>,
}

impl FeatureLayers {
    pub fn new(
        tags: &[String],
        device: &Device,
        camera_node: &Rc<RefCell<SceneTree>>,
        pipeline_provider: &PipeLineProvider,
        style_store: &StyleStore,
    ) -> FeatureLayers {
        let mut layers = FeatureLayers {
            shape_layers: LinkedHashMap::new(),
        };

        tags.into_iter().for_each(|tag| {
            let shape_layer = MeshLayer::new(
                &device,
                include_wgsl!("../shaders/shape_shader.wgsl"),
                Rc::new([ShapeVertex::desc(), InstancePos::desc()]),
                pipeline_provider.clone(),
                None,
                CompareFunction::Always,
            );

            let shape_layer: StyleAdapterNode<MeshLayer> = StyleAdapterNode::new(
                device,
                style_store.subscribe(),
                shape_layer,
                SHADER_STYLE_GROUP_INDEX,
                CompareFunction::Always,
            );

            let layer = camera_node
                .borrow_mut()
                .add_child_with_key(shape_layer, format!("feature_layer {tag}").to_string());
            layers.shape_layers.insert(tag.clone(), layer);
        });

        layers
    }

    pub fn clear_by_key(&mut self, key: String) {
        self.shape_layers.values().for_each(|layer| {
            layer.borrow_mut().clear_by_key(key.clone());
        });
    }

    pub fn get_layer(&mut self, tag: &String) -> Option<Rc<RefCell<SceneTree>>> {
        self.shape_layers.get(tag).cloned()
    }
}
