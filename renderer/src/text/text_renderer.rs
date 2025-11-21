use crate::camera::ScreenPositionCalculator;
use crate::collision_handler::CollisionHandler;
use crate::draw_commands::geometry_to_mesh;
use crate::mesh::mesh::Mesh;
use crate::text::glyph_tesselator::GlyphTesselator;
use crate::vertex_attrs::InstancePos;
use cgmath::num_traits::clamp;
use cgmath::{Deg, Matrix4, Vector2, Vector3};
use geo_types::point;
use rstar::primitives::Rectangle;
use rustybuzz::ttf_parser::GlyphId;
use rustybuzz::{UnicodeBuffer, ttf_parser};
use std::collections::HashMap;
use std::mem;
use wgpu::util::DeviceExt;
use wgpu::{Buffer, Color, Device, RenderPass};
use wgpu_text::TextBrush;
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::glyph_brush::{OwnedSection, OwnedText};

#[derive(Clone)]
pub struct GlyphData {
    pub glyph_id: GlyphId,
    pub rotation: f32,
    pub offset: Vector2<f32>,
}

pub struct TextNodeData {
    pub id: u64,
    pub world_position: Vector3<f32>,
    pub screen_offset: Vector2<f32>,
    pub text: OwnedText,
}

pub struct TextRenderer {
    pub text_brush: TextBrush<FontRef<'static>>,
    id_to_alpha_map: HashMap<u64, f32>,
    sections: Vec<OwnedSection>,

    glyph_mesh_map: HashMap<GlyphId, Mesh>,
    glyph_data: HashMap<GlyphId, Vec<GlyphData>>,
    dirty: bool,
    instance_buffer_map: HashMap<GlyphId, Buffer>,
}

impl TextRenderer {
    const FADE_ANIM_SPEED: f32 = 0.05;
    pub fn new(brush: TextBrush<FontRef<'static>>, device: &Device) -> TextRenderer {
        let face = ttf_parser::Face::parse(include_bytes!("../font.ttf"), 0).unwrap();
        let face = rustybuzz::Face::from_face(face);

        let mut buffer = UnicodeBuffer::new();
        buffer.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        buffer.guess_segment_properties();

        let glyph_buffer = rustybuzz::shape(&face, &[], buffer);

        let mut glyph_mesh_map = HashMap::new();
        for index in 0..glyph_buffer.len() {
            let glyph_info = glyph_buffer.glyph_infos()[index];
            let mut path_builder = GlyphTesselator::new(0.01);
            face.outline_glyph(GlyphId(glyph_info.glyph_id as u16), &mut path_builder);
            let glyph_buf = path_builder.tessellate_fill(Vector2::new(0.0, 0.0f32), Color::RED);
            glyph_mesh_map.insert(
                GlyphId(glyph_info.glyph_id as u16),
                geometry_to_mesh(device, &glyph_buf),
            );
        }

        TextRenderer {
            text_brush: brush,
            id_to_alpha_map: HashMap::new(),
            sections: vec![],

            glyph_mesh_map,
            glyph_data: HashMap::new(),
            dirty: false,
            instance_buffer_map: HashMap::new(),
        }
    }

    pub fn insert2(&mut self, glyph_data: Vec<GlyphData>) {
        glyph_data.into_iter().for_each(|item| {
            self.glyph_data
                .entry(item.glyph_id)
                .and_modify(|list| {
                    list.push(item.clone());
                })
                .or_insert(vec![item.clone()]);
        });
        self.dirty = true;
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
        let mut section = OwnedSection::default()
            .add_text(data.text.clone())
            .with_screen_position((
                screen_pos.x as f32 + data.screen_offset.x,
                screen_pos.y as f32 + data.screen_offset.y,
            ));

        let section_rect = self.text_brush.glyph_bounds(&section).unwrap();
        let center_offset = section_rect.width() * 0.5;
        section.screen_position.0 -= center_offset;
        let section_rect = Rectangle::from_corners(
            point! { x: section_rect.min.x - center_offset, y: section_rect.min.y},
            point! { x: section_rect.max.x - center_offset, y: section_rect.max.y},
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

    fn update_attrs(&mut self, device: &Device) {
        self.glyph_data.iter().for_each(|(key, list)| {
            let mut attrs = vec![];
            list.iter().for_each(|glyph_data| {
                let rotation_matrix = Matrix4::<f64>::from_angle_z(Deg(glyph_data.rotation as f64));
                let matrix = rotation_matrix;

                let instance_pos = InstancePos {
                    position: Vector3::new(glyph_data.offset.x, glyph_data.offset.y, 0.0).into(),
                    color_alpha: 1.0,
                    matrix: matrix.cast().unwrap().into(),
                    bbox: [0.0, 0.0, 0.0, 0.0],
                };
                attrs.push(instance_pos);
            });

            let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(attrs.as_slice()),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            self.instance_buffer_map
                .insert(key.clone(), instance_buffer);
        });
    }

    pub fn render(
        &mut self,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        render_pass: &mut RenderPass,
    ) {
        self.id_to_alpha_map.clear();

        if self.dirty {
            self.update_attrs(device);
            self.dirty = false;
        }

        if !self.instance_buffer_map.is_empty() && !self.glyph_data.is_empty() {
            self.glyph_data.iter().for_each(|(glyph_id, list)| {
                let mesh = self.glyph_mesh_map.get(glyph_id).unwrap();
                let v_buf = mesh.vertex_buf.get(0).unwrap();
                let i_buf = mesh.index_buf.get(0).unwrap();
                let instance_buffer = self.instance_buffer_map.get(glyph_id).unwrap();

                render_pass.set_vertex_buffer(0, v_buf.slice(..));
                render_pass.set_index_buffer(i_buf.0.slice(..), wgpu::IndexFormat::Uint32);

                render_pass.set_vertex_buffer(1, instance_buffer.slice(..));

                render_pass.draw_indexed(0..i_buf.1 as u32, 0, 0..list.len() as u32);
            });
        }

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
