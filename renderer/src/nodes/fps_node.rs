use crate::fps::FpsCounter;
use crate::nodes::scene_tree::RenderContext;
use crate::nodes::SceneNode;
use crate::text::text_renderer::TextNodeData;
use crate::GlobalContext;
use cgmath::{vec2, vec3};
use wgpu::{Device, Queue};

pub struct FpsNode {
    counter: FpsCounter<100>,
}

impl FpsNode {
    pub fn new() -> Self {
        Self {
            counter: FpsCounter::new(),
        }
    }
}

impl SceneNode for FpsNode {
    fn setup(&mut self, _render_context: &mut RenderContext, _device: &Device) {}

    fn update(
        &mut self,
        _device: &Device,
        _queue: &Queue,
        global_context: &mut GlobalContext,
    ) {
        global_context.text_renderer.insert(
            &mut TextNodeData {
                id: 0,
                text: format!("FPS {}", self.counter.update() as i32),
                size: 40.0,
                alpha: 1.0,
                positions: vec![vec3(100.0, 120.0, 0.0)],
                screen_offset: vec2(0.0, 0.0),
                screen_space: true,
                glyph_buffer: None,
            },
            &mut global_context.collision_handler,
            &global_context.view_projection
        )
    }
}
