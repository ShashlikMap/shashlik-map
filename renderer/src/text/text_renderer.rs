use crate::camera::ScreenPositionCalculator;
use crate::collision_handler::CollisionHandler;
use crate::draw_commands::geometry_to_mesh;
use crate::mesh::mesh::Mesh;
use crate::text::glyph_tesselator::GlyphTesselator;
use crate::vertex_attrs::InstancePos;
use cgmath::num_traits::clamp;
use cgmath::{Matrix4, Vector2, Vector3};
use geo_types::point;
use rstar::primitives::Rectangle;
use rustybuzz::ttf_parser::GlyphId;
use rustybuzz::{ttf_parser, Direction, Face, ShapePlan, UnicodeBuffer};
use std::collections::HashMap;
use wgpu::util::DeviceExt;
use wgpu::{Buffer, Color, Device, RenderPass};

#[derive(Clone)]
pub struct GlyphData {
    pub glyph_id: GlyphId,
    pub rotation: f32,
    pub position: (f32, f32),
    pub alpha: f32,
    pub offset: Vector2<f32>,
}

pub struct TextNodeData {
    pub id: u64,
    pub textv: String,
    pub alpha: f32,
    pub world_position: Vector3<f32>,
    pub screen_offset: Vector2<f32>,
}

pub struct TextRenderer {
    id_to_alpha_map: HashMap<u64, f32>,

    face: Face<'static>,
    face_shape_plan: ShapePlan,

    glyph_mesh_map: HashMap<GlyphId, Mesh>,
    glyph_data: HashMap<GlyphId, Vec<GlyphData>>,
    instance_buffer_map: HashMap<GlyphId, Buffer>,
}

impl TextRenderer {
    const SCALE: f32 = 0.04;
    const FADE_ANIM_SPEED: f32 = 0.05;
    pub fn new(device: &Device) -> TextRenderer {
        let face = ttf_parser::Face::parse(include_bytes!("../font.ttf"), 0).unwrap();
        let face = rustybuzz::Face::from_face(face);

        let mut buffer = UnicodeBuffer::new();
        buffer.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        buffer.guess_segment_properties();


        let face_shape_plan = ShapePlan::new(&face, Direction::LeftToRight, Some(buffer.script()), None, &[]);

        let glyph_buffer = rustybuzz::shape(&face, &[], buffer);

        let mut glyph_mesh_map = HashMap::new();
        for index in 0..glyph_buffer.len() {
            let glyph_info = glyph_buffer.glyph_infos()[index];
            let mut path_builder = GlyphTesselator::new(Self::SCALE);
            face.outline_glyph(GlyphId(glyph_info.glyph_id as u16), &mut path_builder);
            let glyph_buf = path_builder.tessellate_fill(Vector2::new(0.0, 0.0f32), Color::RED);
            glyph_mesh_map.insert(
                GlyphId(glyph_info.glyph_id as u16),
                geometry_to_mesh(device, &glyph_buf),
            );
        }

        TextRenderer {
            id_to_alpha_map: HashMap::new(),
            face,
            face_shape_plan,
            glyph_mesh_map,
            glyph_data: HashMap::new(),
            instance_buffer_map: HashMap::new(),
        }
    }

    pub fn insert(
        &mut self,
        data: &mut TextNodeData,
        config: &wgpu::SurfaceConfiguration,
        collision_handler: &mut CollisionHandler,
        screen_position_calculator: &ScreenPositionCalculator,
    ) {
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(data.textv.as_str());
        buffer.guess_segment_properties();

        let glyph_buffer = rustybuzz::shape_with_plan(&self.face, &self.face_shape_plan, buffer);
        let glyphs_positions = glyph_buffer.glyph_positions();
        let glyphs_infos = glyph_buffer.glyph_infos();
        let mut pos = 0.0;

        let width = glyph_buffer
            .glyph_positions()
            .iter()
            .fold(0, |aggr, glyph| aggr + glyph.x_advance) as f32 * Self::SCALE;
        let height = (self.face.ascender() + self.face.descender()) as f32 * Self::SCALE;

        let origin = screen_position_calculator.screen_position(data.world_position.cast().unwrap());
        let section_rect = Rectangle::from_corners(
            point! { x: origin.x as f32, y: origin.y as f32 },
            point! { x: origin.x as f32 + width, y: origin.y as f32 + height },
        );

        let within_screen = collision_handler.within_screen(config, section_rect);
        if within_screen {
            let contains = self.id_to_alpha_map.contains_key(&data.id);
            let mut alpha = *self
                .id_to_alpha_map
                .entry(data.id)
                .or_insert(data.alpha);
            if contains {
                data.alpha = alpha;
                return;
            }

            if collision_handler.insert(section_rect) {
                alpha = clamp(alpha + Self::FADE_ANIM_SPEED, 0.0, 1.0);
            } else {
                alpha = clamp(alpha - Self::FADE_ANIM_SPEED, 0.0, 1.0);
            }
            data.alpha = alpha;

            for index in 0..glyph_buffer.len() {
                let position = glyphs_positions[index];
                let glyph_info = glyphs_infos[index];

                let item = GlyphData {
                    glyph_id: GlyphId(glyph_info.glyph_id as u16),
                    rotation: 0.0,
                    alpha,
                    position: (data.world_position.x, data.world_position.y).into(),
                    offset: Vector2::new(pos, 0.0),
                };
                self.glyph_data
                    .entry(item.glyph_id)
                    .and_modify(|list| {
                        list.push(item.clone());
                    })
                    .or_insert(vec![item.clone()]);

                pos += position.x_advance as f32 * Self::SCALE;
            }
        }
    }

    fn update_attrs(&mut self, device: &Device, config: &wgpu::SurfaceConfiguration,) {
        self.instance_buffer_map.clear();
        self.glyph_data.iter().for_each(|(key, list)| {
            let mut attrs = vec![];
            list.iter().for_each(|glyph_data| {
                // let rotation_matrix = Matrix4::<f64>::from_angle_z(Deg(glyph_data.rotation as f64));
                // let matrix = rotation_matrix;
                let matrix = Matrix4::<f32>::from_translation(Vector3::new(glyph_data.offset.x, 0.0, 0.0));

                let instance_pos = InstancePos {
                    position: Vector3::new(glyph_data.position.0, glyph_data.position.1, 0.0).into(),
                    color_alpha: glyph_data.alpha,
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
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        render_pass: &mut RenderPass,
    ) {
        self.id_to_alpha_map.clear();

        self.update_attrs(device, config);

        if !self.instance_buffer_map.is_empty() && !self.glyph_data.is_empty() {
            self.glyph_data.iter().for_each(|(glyph_id, list)| {
                if self.glyph_mesh_map.contains_key(glyph_id) {
                    let mesh = self.glyph_mesh_map.get(glyph_id).unwrap();
                    let v_buf = mesh.vertex_buf.get(0).unwrap();
                    let i_buf = mesh.index_buf.get(0).unwrap();
                    let instance_buffer = self.instance_buffer_map.get(glyph_id).unwrap();

                    render_pass.set_vertex_buffer(0, v_buf.slice(..));
                    render_pass.set_index_buffer(i_buf.0.slice(..), wgpu::IndexFormat::Uint32);

                    render_pass.set_vertex_buffer(1, instance_buffer.slice(..));

                    render_pass.draw_indexed(0..i_buf.1 as u32, 0, 0..list.len() as u32);
                }
            });
        }

        self.glyph_data.clear();
    }
}
