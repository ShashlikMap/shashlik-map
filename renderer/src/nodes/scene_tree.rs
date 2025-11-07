use crate::nodes::SceneNode;
use crate::GlobalContext;
use std::cell::RefCell;
use std::rc::Rc;
use wgpu::{BindGroupLayout, CompareFunction, Device, Queue, RenderPass};

#[derive(Clone)]
pub struct RenderContext {
    pub bind_group_layouts: Vec<BindGroupLayout>,
    pub can_write_depth: bool,
    pub depth_compare: CompareFunction,
}

impl Default for RenderContext {
    fn default() -> Self {
        RenderContext {
            bind_group_layouts: vec![],
            can_write_depth: true,
            depth_compare: CompareFunction::Less,
        }
    }
}

pub struct SceneTree {
    children: Vec<Rc<RefCell<SceneTree>>>,
    pub value: Box<dyn SceneNode>,
    key: String,
}

impl SceneTree {
    pub fn new(value: impl SceneNode + 'static, key: String) -> Self {
        SceneTree {
            children: vec![],
            value: Box::new(value),
            key,
        }
    }

    pub fn add_child(&mut self, value: impl SceneNode + 'static) -> Rc<RefCell<SceneTree>> {
        self.add_child_with_key(value, "".to_string())
    }
    pub fn add_child_with_key(
        &mut self,
        value: impl SceneNode + 'static,
        key: String,
    ) -> Rc<RefCell<SceneTree>> {
        let node_ref = Rc::new(RefCell::new(SceneTree::new(value, key)));
        self.children.push(node_ref.clone());
        node_ref
    }

    pub fn clear_by_key(&mut self, key: String) {
        self.children.retain(|node| node.borrow().key != key);
        // println!(
        //     "layer {:?}, key_to_remove:{:?}, count: {}",
        //     self.key,
        //     key,
        //     self.children.len()
        // );
    }

    pub fn clear(&mut self) {
        self.children.clear();
    }
}

impl SceneNode for SceneTree {
    fn setup(&mut self, render_context: &mut RenderContext, device: &Device) {
        self.children.iter().for_each(|scene_node| {
            // clone the context so the subtree gets its own version
            let mut cloned_context = render_context.clone();

            scene_node
                .borrow_mut()
                .value
                .setup(&mut cloned_context, device);
            scene_node.borrow_mut().setup(&mut cloned_context, device);
        });
    }

    fn update(
        &mut self,
        device: &Device,
        queue: &Queue,
        config: &wgpu::SurfaceConfiguration,
        global_context: &mut GlobalContext,
    ) {
        self.children.iter().for_each(|scene_node| {
            scene_node
                .borrow_mut()
                .value
                .update(device, &queue, config, global_context);
            scene_node
                .borrow_mut()
                .update(device, queue, config, global_context);
        });
    }

    fn render(&self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        self.children.iter().for_each(|scene_node| {
            scene_node
                .borrow_mut()
                .value
                .render(render_pass, global_context);
            scene_node.borrow_mut().render(render_pass, global_context);
        });
    }

    fn resize(&mut self, width: u32, height: u32, queue: &Queue) {
        self.children.iter().for_each(|scene_node| {
            scene_node.borrow_mut().value.resize(width, height, queue);
            scene_node.borrow_mut().resize(width, height, queue);
        });
    }
}
