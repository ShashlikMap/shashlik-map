use crate::GlobalContext;
use crate::geometry_data::TextData;
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::SceneNode;
use crate::nodes::scene_tree::RenderContext;
use crate::text::text_renderer::TextNodeData;
use wgpu::{Device, Queue};
use wgpu_text::glyph_brush::OwnedText;

pub struct TextLayer;

impl SceneNode for TextLayer {}

pub struct TextNode {
    data: Vec<TextNodeData>,
}

impl TextNode {
    pub fn new(text_data: Vec<TextData>, spatial_data: SpatialData) -> Self {
        Self {
            data: text_data
                .iter()
                .map(|item| {
                    let owned_text = OwnedText::new(item.text.as_str())
                        .with_scale(40.0)
                        .with_color([0.0, 0.0, 0.0, 0.0]);
                    TextNodeData {
                        id: item.id.clone(),
                        // text node doesn't have to be super precise
                        world_position: item.position + spatial_data.transform.cast().unwrap(),
                        screen_offset: item.screen_offset,
                        text: owned_text,
                    }
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
            .camera_controller
            .borrow()
            .screen_position_calculator(config);
        self.data.iter_mut().for_each(|item| {
            global_context.text_renderer.insert(
                item,
                config,
                &mut global_context.collision_handler,
                &screen_position_calculator,
            )
        });
    }
}
