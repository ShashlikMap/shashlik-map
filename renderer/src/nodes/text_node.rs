use crate::GlobalContext;
use crate::geometry_data::TextData;
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::SceneNode;
use crate::nodes::scene_tree::RenderContext;
use crate::text::text_renderer::TextNodeData;
use wgpu::{Device, Queue};

pub struct TextNode {
    pub data: Vec<TextNodeData>,
}

impl TextNode {
    pub fn new(text_data: Vec<TextData>, spatial_data: SpatialData) -> Self {
        Self {
            data: text_data
                .into_iter()
                .map(|item| TextNodeData {
                    id: item.id,
                    text: item.text,
                    size: item.size,
                    alpha: 0.0,
                    positions: item
                        .positions
                        .iter()
                        .map(|pos| pos + spatial_data.transform.cast().unwrap())
                        .collect(),
                    screen_offset: item.screen_offset,
                    glyph_buffer: None,
                })
                .collect(),
        }
    }
}

impl SceneNode for TextNode {
    fn setup(&mut self, _render_context: &mut RenderContext, _device: &Device) {}

    fn update(
        &mut self,
        _device: &Device,
        _queue: &Queue,
        config: &wgpu::SurfaceConfiguration,
        global_context: &mut GlobalContext,
    ) {
        let screen_position_calculator = global_context
            .view_projection
            .screen_position_calculator(config);
        self.data.iter_mut().for_each(|item| {
            global_context.text_renderer.insert(
                item,
                &mut global_context.collision_handler,
                &screen_position_calculator,
            )
        });
    }
}
