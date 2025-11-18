use crate::nodes::scene_tree::SceneTree;
use crate::nodes::shape_layers::ShapeLayers;
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) struct Layers {
    shape_layers: ShapeLayers,
    pub mesh_layer: Rc<RefCell<SceneTree>>,
    pub screen_shape_layer: Rc<RefCell<SceneTree>>,
    pub text_layer: Rc<RefCell<SceneTree>>,
}

impl Layers {
    pub fn new(
        shape_layers: ShapeLayers,
        mesh_layer: Rc<RefCell<SceneTree>>,
        screen_shape_layer: Rc<RefCell<SceneTree>>,
        text_layer: Rc<RefCell<SceneTree>>,
    ) -> Layers {
        Layers {
            shape_layers,
            mesh_layer,
            screen_shape_layer,
            text_layer,
        }
    }
    pub fn shape_layers(&self, index: usize) -> Rc<RefCell<SceneTree>> {
        self.shape_layers.get_shape_layer(index)
    }
    pub fn clear(&mut self, key: String) {
        self.mesh_layer.borrow_mut().clear_by_key(key.clone());
        self.shape_layers.clear_by_key(key.clone());
        self.screen_shape_layer
            .borrow_mut()
            .clear_by_key(key.clone());
        self.text_layer.borrow_mut().clear_by_key(key.clone());
    }
}
