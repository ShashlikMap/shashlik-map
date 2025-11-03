use crate::geometry_data::TextData;
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::scene_tree::RenderContext;
use crate::nodes::SceneNode;
use crate::GlobalContext;
use cgmath::num_traits::clamp;
use cgmath::Vector3;
use geo_types::point;
use rstar::primitives::Rectangle;
use wgpu::{Device, Queue, RenderPass};
use wgpu_text::glyph_brush::{Section, Text};

pub struct TextLayer;

impl SceneNode for TextLayer {
    fn render(&self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        global_context.text_brush.draw(render_pass);
    }
}

struct TextNodeData {
    world_position: Vector3<f32>,
    text: String,
    alpha: f32,
}

pub struct TextNode {
    data: Vec<TextNodeData>,
}

impl TextNode {
    const FADE_ANIM_SPEED: f32 = 0.05;

    pub fn new(text_data: Vec<TextData>, spatial_data: SpatialData) -> Self {
        Self {
            data: text_data
                .iter()
                .map(|item| TextNodeData {
                    world_position: item.position + spatial_data.transform,
                    text: item.text.clone(),
                    alpha: 0.0,
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
            let screen_pos = screen_position_calculator.screen_position(item.world_position);

            let section = Section::default()
                .add_text(
                    Text::new(item.text.as_str())
                        .with_scale(40.0)
                        .with_color([0.0, 0.0, 0.0, item.alpha]),
                )
                .with_screen_position((screen_pos.x, screen_pos.y));

            let section_rect = global_context.text_brush.glyph_bounds(&section).unwrap();
            let section_rect = Rectangle::from_corners(
                point! { x: section_rect.min.x, y: section_rect.min.y},
                point! { x: section_rect.max.x, y: section_rect.max.y},
            );
            if let Some(added) = global_context
                .collision_handler
                .insert(config, section_rect)
            {
                if added {
                    item.alpha = clamp(item.alpha + Self::FADE_ANIM_SPEED, 0.0, 1.0);
                } else {
                    item.alpha = clamp(item.alpha - Self::FADE_ANIM_SPEED, 0.0, 1.0);
                }
                global_context.text_sections.push(section.to_owned());
            }
        });
    }
}
