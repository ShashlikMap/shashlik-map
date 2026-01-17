use crate::nodes::feature_layers::FeatureLayers;
use crate::nodes::scene_tree::SceneTree;
use crate::nodes::shape_layers::ShapeLayers;
use std::cell::RefCell;
use std::rc::Rc;
use wgpu::Device;
use crate::mesh_layers::general_mesh_layer::GeneralMeshLayer;

pub(crate) struct Layers {
    shape_layers: ShapeLayers,
    feature_layers: FeatureLayers,
    pub mesh_layer: Rc<RefCell<SceneTree>>,
    pub screen_shape_layer: Rc<RefCell<SceneTree>>,
    pub text_layer: Rc<RefCell<SceneTree>>,
    pub new_mesh_layer: GeneralMeshLayer
}

impl Layers {
    pub fn new(
        device: &Device,
        shape_layers: ShapeLayers,
        feature_layers: FeatureLayers,
        mesh_layer: Rc<RefCell<SceneTree>>,
        screen_shape_layer: Rc<RefCell<SceneTree>>,
        text_layer: Rc<RefCell<SceneTree>>,
    ) -> Layers {
        Layers {
            shape_layers,
            feature_layers,
            mesh_layer,
            screen_shape_layer,
            text_layer,
            new_mesh_layer: GeneralMeshLayer::new(device)
        }
    }
    pub fn shape_layers(&self, index: usize) -> Rc<RefCell<SceneTree>> {
        self.shape_layers.get_shape_layer(index)
    }

    pub fn feature_layers(&mut self, tag: &String) -> Option<Rc<RefCell<SceneTree>>> {
        self.feature_layers.get_layer(tag)
    }

    pub fn clear(&mut self, key: String) {
        self.mesh_layer.borrow_mut().clear_by_key(key.clone());
        self.shape_layers.clear_by_key(key.clone());
        self.screen_shape_layer
            .borrow_mut()
            .clear_by_key(key.clone());
        self.text_layer.borrow_mut().clear_by_key(key.clone());
        self.feature_layers.clear_by_key(key.clone());
    }
}
