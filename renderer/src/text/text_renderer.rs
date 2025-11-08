use crate::camera::ScreenPositionCalculator;
use crate::collision_handler::CollisionHandler;
use cgmath::Vector3;
use cgmath::num_traits::clamp;
use geo_types::point;
use rstar::primitives::Rectangle;
use std::collections::HashMap;
use std::mem;
use wgpu::RenderPass;
use wgpu_text::TextBrush;
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::glyph_brush::{OwnedSection, OwnedText};

pub struct TextNodeData {
    pub id: u64,
    pub world_position: Vector3<f32>,
    pub text: OwnedText,
}

pub struct TextRenderer {
    pub text_brush: TextBrush<FontRef<'static>>,
    id_to_alpha_map: HashMap<u64, f32>,
    sections: Vec<OwnedSection>,
}

impl TextRenderer {
    const FADE_ANIM_SPEED: f32 = 0.05;
    pub fn new(brush: TextBrush<FontRef<'static>>) -> TextRenderer {
        TextRenderer {
            text_brush: brush,
            id_to_alpha_map: HashMap::new(),
            sections: vec![],
        }
    }

    pub fn insert(
        &mut self,
        data: &mut TextNodeData,
        config: &wgpu::SurfaceConfiguration,
        collision_handler: &mut CollisionHandler,
        screen_position_calculator: &ScreenPositionCalculator,
    ) {
        let screen_pos =
            screen_position_calculator.screen_position(data.world_position.cast().unwrap());
        let section = OwnedSection::default()
            .add_text(data.text.clone())
            .with_screen_position((screen_pos.x as f32, screen_pos.y as f32));

        let section_rect = self.text_brush.glyph_bounds(&section).unwrap();
        let section_rect = Rectangle::from_corners(
            point! { x: section_rect.min.x, y: section_rect.min.y},
            point! { x: section_rect.max.x, y: section_rect.max.y},
        );
        let within_screen = collision_handler.within_screen(config, section_rect);
        if within_screen {
            let contains = self.id_to_alpha_map.contains_key(&data.id);
            let mut alpha = *self
                .id_to_alpha_map
                .entry(data.id)
                .or_insert(data.text.extra.color[3]);
            if contains {
                data.text.extra.color[3] = alpha;
                return;
            }

            if collision_handler.insert(section_rect) {
                alpha = clamp(alpha + Self::FADE_ANIM_SPEED, 0.0, 1.0);
            } else {
                alpha = clamp(alpha - Self::FADE_ANIM_SPEED, 0.0, 1.0);
            }
            data.text.extra.color[3] = alpha;

            self.sections.push(section);
        }
    }

    pub fn render(
        &mut self,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        render_pass: &mut RenderPass,
    ) {
        self.id_to_alpha_map.clear();
        self.text_brush
            .queue(
                &device,
                &queue,
                mem::take(&mut self.sections)
                    .iter()
                    .map(|item| item.to_borrowed())
                    .collect::<Vec<_>>(),
            )
            .unwrap();

        self.text_brush.draw(render_pass);
    }
}
