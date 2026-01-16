use crate::GlobalContext;
use crate::geometry_data::TextData;
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::SceneNode;
use crate::nodes::scene_tree::RenderContext;
use wgpu::{Device, Queue};

pub struct TextNode {
    pub data: Vec<TextData>,
}

impl TextNode {
    pub fn new(mut text_data: Vec<TextData>, spatial_data: SpatialData) -> Self {
        text_data.iter_mut().for_each(|item| {
            item.alpha = 0.0;
            item.positions = item
                .positions
                .iter()
                .map(|pos| pos + spatial_data.transform.cast().unwrap())
                .collect()
        });
        Self { data: text_data }
    }
}

impl SceneNode for TextNode {
    fn setup(&mut self, _render_context: &mut RenderContext, _device: &Device) {}

    fn update(&mut self, _device: &Device, _queue: &Queue, global_context: &mut GlobalContext) {
        self.data.iter_mut().for_each(|item| {
            global_context.text_renderer.insert(
                item,
                &mut global_context.collision_handler,
                &global_context.view_projection,
            )
        });
    }
}
