use crate::GlobalContext;
use crate::camera::{FLIP_Y, OPENGL_TO_WGPU_MATRIX};
use crate::geometry_data::TextData;
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::SceneNode;
use crate::nodes::scene_tree::RenderContext;
use cgmath::num_traits::clamp;
use cgmath::{Vector3, Vector4};
use geo_types::{coord, point};
use rstar::RTreeObject;
use rstar::primitives::{GeomWithData, Rectangle};
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

        let matrix = FLIP_Y
            * OPENGL_TO_WGPU_MATRIX
            * global_context.camera_controller.borrow().cached_matrix;
        self.data.iter_mut().for_each(|item| {
            let pos = matrix * Vector4::new(item.world_position.x, item.world_position.y, 0.0, 1.0);
            let clip_pos_x = pos.x / pos.w;
            let clip_pos_y = pos.y / pos.w;
            if clip_pos_x >= -1.1 && clip_pos_x <= 1.1 && clip_pos_y >= -1.1 && clip_pos_y <= 1.1 {
                let screen_size = (config.width as f32, config.height as f32);
                let screen_pos = coord! {x:  screen_size.0 * (clip_pos_x + 1.0) / 2.0,
                y:   screen_size.1 - (screen_size.1 * (clip_pos_y + 1.0) / 2.0)};

                let section = Section::default()
                    .add_text(
                        Text::new(item.text.as_str())
                            .with_scale(40.0)
                            .with_color([0.0, 0.0, 0.0, item.alpha]),
                    )
                    .with_screen_position((screen_pos.x, screen_pos.y));

                let section_rect = global_context.text_brush.glyph_bounds(&section).unwrap();
                let section_rect_envelope = Rectangle::from_corners(
                    point! { x: section_rect.min.x, y: section_rect.min.y},
                    point! { x: section_rect.max.x, y: section_rect.max.y},
                )
                .envelope();
                let count = global_context
                    .text_sections
                    .locate_in_envelope_intersecting(&section_rect_envelope)
                    .count();

                let to_add = if count <= 0 {
                    item.alpha = clamp(item.alpha + Self::FADE_ANIM_SPEED, 0.0, 1.0);
                    true
                } else {
                    item.alpha = clamp(item.alpha - Self::FADE_ANIM_SPEED, 0.0, 1.0);
                    item.alpha > 0.0
                };

                if to_add {
                    global_context.text_sections.insert(GeomWithData::new(
                        Rectangle::from(section_rect_envelope),
                        section.to_owned(),
                    ))
                }
            }
        });
    }
}
