use crate::camera::ScreenPositionCalculator;
use crate::collision_handler::CollisionHandler;
use cgmath::Vector3;
use cgmath::num_traits::clamp;
use geo_types::point;
use rstar::primitives::Rectangle;
use std::collections::{HashMap, HashSet};
use std::mem;
use wgpu::RenderPass;
use wgpu_text::TextBrush;
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::glyph_brush::{OwnedSection, OwnedText};

pub struct TextNodeData2 {
    pub id: u64,
    pub world_position: Vector3<f32>,
    pub text: OwnedText,
}

pub struct TextRenderer {
    pub text_brush: TextBrush<FontRef<'static>>,
    sss: HashMap<u64, f32>,
    sections: Vec<OwnedSection>,
}

impl TextRenderer {
    pub fn new(brush: TextBrush<FontRef<'static>>) -> TextRenderer {
        TextRenderer {
            text_brush: brush,
            sss: HashMap::new(),
            sections: vec![],
        }
    }

    pub fn insert(
        &mut self,
        data: &mut TextNodeData2,
        config: &wgpu::SurfaceConfiguration,
        collision_handler: &mut CollisionHandler,
        screen_position_calculator: &ScreenPositionCalculator,
    ) {
        if data.text.text == "Nagano" {
            println!("added alpha = {}", data.text.extra.color[3]);
        }
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
            let cont = self.sss.contains_key(&data.id);
            let mut alpha = *self.sss.entry(data.id).or_insert(data.text.extra.color[3]);
            if cont {
                data.text.extra.color[3] = 1.0;
                // self.sections.push(section);
                return;
            }

            if collision_handler.insert2(section_rect) {
                if data.text.text == "Nagano" {
                    println!("qqq");
                }
                alpha = clamp(alpha + 0.05, 0.0, 1.0);
            } else {
                if data.text.text == "Nagano" {
                    println!("bbb");
                }
                alpha = clamp(alpha - 0.05, 0.0, 1.0);
            }
            if data.text.text == "Nagano" {
                println!("hhh = {}", alpha);
                println!("aaa = {}", data.text.extra.color[3]);
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
        self.sss.clear();
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
