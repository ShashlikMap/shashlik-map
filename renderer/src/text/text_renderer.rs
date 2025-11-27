use crate::camera::ScreenPositionCalculator;
use crate::collision_handler::CollisionHandler;
use crate::draw_commands::geometry_to_mesh;
use crate::mesh::mesh::Mesh;
use crate::text::glyph_tesselator::GlyphTesselator;
use crate::vertex_attrs::InstancePos;
use cgmath::num_traits::clamp;
use cgmath::{Deg, InnerSpace, Matrix4, Quaternion, Rotation, Vector2, Vector3};
use geo_types::{Coord, coord, point};
use rstar::primitives::Rectangle;
use rustc_hash::FxHashMap;
use rustybuzz::ttf_parser::GlyphId;
use rustybuzz::{Direction, Face, GlyphBuffer, ShapePlan, UnicodeBuffer, ttf_parser};
use std::collections::HashMap;
use wgpu::util::DeviceExt;
use wgpu::{Buffer, Color, Device, RenderPass};

#[derive(Clone)]
pub struct GlyphData {
    pub glyph_id: GlyphId,
    pub position: (f32, f32),
    pub alpha: f32,
    pub matrix: Matrix4<f32>,
}

pub struct TextNodeData {
    pub id: u64,
    pub text: String,
    pub size: f32,
    pub alpha: f32,
    pub world_position: Vector3<f32>,
    pub positions: Option<Vec<Vector3<f32>>>,
    pub screen_offset: Vector2<f32>,
    pub glyph_buffer: Option<GlyphBuffer>,
}

pub struct TextRenderer {
    id_to_alpha_map: HashMap<u64, f32>,

    face: Face<'static>,
    face_shape_plan: ShapePlan,

    glyph_mesh_map: FxHashMap<GlyphId, Mesh>,
    glyph_data: FxHashMap<GlyphId, Vec<GlyphData>>,
    instance_buffer_map: FxHashMap<GlyphId, Buffer>,
}

impl TextRenderer {
    const MAX_SCALE: f32 = 0.035;
    const FADE_ANIM_SPEED: f32 = 0.05;
    pub fn new(device: &Device) -> TextRenderer {
        let face = ttf_parser::Face::parse(include_bytes!("../font.ttf"), 0).unwrap();
        let face = rustybuzz::Face::from_face(face);

        let mut buffer = UnicodeBuffer::new();
        buffer.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ-");
        buffer.guess_segment_properties();

        let face_shape_plan = ShapePlan::new(
            &face,
            Direction::LeftToRight,
            Some(buffer.script()),
            None,
            &[],
        );

        let glyph_buffer = rustybuzz::shape(&face, &[], buffer);

        let mut glyph_mesh_map = FxHashMap::default();
        for index in 0..glyph_buffer.len() {
            let glyph_info = glyph_buffer.glyph_infos()[index];
            let mut path_builder = GlyphTesselator::new(Self::MAX_SCALE);
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
            glyph_data: FxHashMap::default(),
            instance_buffer_map: FxHashMap::default(),
        }
    }

    pub fn insert(
        &mut self,
        data: &mut TextNodeData,
        config: &wgpu::SurfaceConfiguration,
        collision_handler: &mut CollisionHandler,
        screen_position_calculator: &ScreenPositionCalculator,
    ) {
        let glyph_buffer = data.glyph_buffer.get_or_insert_with(|| {
            let mut buffer = UnicodeBuffer::new();
            buffer.push_str(data.text.as_str());
            buffer.guess_segment_properties();
            rustybuzz::shape_with_plan(&self.face, &self.face_shape_plan, buffer)
        });

        let glyphs_positions = glyph_buffer.glyph_positions();
        let glyphs_infos = glyph_buffer.glyph_infos();
        let mut glyph_total_xadvance = 0.0;

        let units = self.face.units_per_em() as f32;
        let scale = data.size / units;

        let width = glyph_buffer
            .glyph_positions()
            .iter()
            .fold(0, |aggr, glyph| aggr + glyph.x_advance) as f32
            * scale;
        let height = (self.face.ascender() + self.face.descender()) as f32 * scale;

        let scale_m = Matrix4::from_scale(scale / Self::MAX_SCALE);

        if let Some(line_positions) = &data.positions {
            let origin =
                screen_position_calculator.screen_position(data.world_position.cast().unwrap());

            let some_middle_point_index = line_positions.len() / 2;
            let mut prev: Option<Coord<f32>> = None;
            let mut glyph_index = 0;
            let glyphs_len = glyph_buffer.len();
            let segments_count = line_positions.len();

            let mut segments_len = 0.0;
            let mut segments_vector = Vector3::new(0.0, 0.0, 0.0);

            let mut glyphs_to_draw = vec![];

            let mut backward = false;

            let flip_rot_m = Matrix4::from_angle_z(Deg(180.0));

            for (index, current) in line_positions[some_middle_point_index..].iter().enumerate() {
                if glyph_index >= glyphs_len {
                    break;
                }

                let current =
                    screen_position_calculator.screen_position(current.cast().unwrap()) - origin;
                let current = coord! {x : current.x as f32, y: current.y as f32 };
                if let Some(prev) = prev {
                    // check if we need to render text backward to
                    if index == 1 {
                        if current.x < prev.x {
                            backward = true;
                        }
                    }
                    let seg_vector = current - prev;
                    let seg_vector = Vector3::new(seg_vector.x, seg_vector.y, 0.0);
                    segments_len += seg_vector.magnitude();

                    let seg_rotation: Quaternion<f32> =
                        Rotation::between_vectors(seg_vector.normalize(), Vector3::unit_x());
                    let half_height_translation =
                        Matrix4::from_translation(Vector3::new(0.0, -height / 2.0, 0.0));
                    let rot_m: Matrix4<f32> = seg_rotation.into();
                    let scale_rot_height_m = scale_m * rot_m * half_height_translation;

                    while glyph_index < glyphs_len {
                        let real_glyph_index = if backward {
                            glyphs_len - glyph_index - 1
                        } else {
                            glyph_index
                        };

                        let position = glyphs_positions[real_glyph_index];
                        if index < segments_count - 1 && segments_vector.magnitude() > segments_len
                        {
                            break;
                        }

                        let x_advance = position.x_advance as f32 * scale;

                        let x_advance_vector = Vector3::new(x_advance, 0.0, 0.0);
                        let glyph_info = glyphs_infos[real_glyph_index];

                        let rotated_glyph_vector = seg_rotation.rotate_vector(x_advance_vector);

                        let matrix = if backward {
                            let x_advance_translation =
                                Matrix4::from_translation(-x_advance_vector);
                            Matrix4::from_translation(segments_vector)
                                * flip_rot_m
                                * scale_rot_height_m
                                * x_advance_translation
                        } else {
                            Matrix4::from_translation(segments_vector) * scale_rot_height_m
                        };

                        let glyph_rect = Rectangle::from_corners(
                            point! { x: origin.x as f32 + segments_vector.x - height, y: origin.y as f32 + segments_vector.y - height },
                            point! { x: origin.x as f32 + segments_vector.x + height, y: origin.y as f32 + segments_vector.y + height},
                        );

                        // FIXME find a root cause, or rstar crashes
                        if glyph_rect.lower().x().is_nan() {
                            // fast exit
                            glyph_index = glyphs_len;
                            break;
                        }

                        segments_vector += rotated_glyph_vector;

                        let item = GlyphData {
                            glyph_id: GlyphId(glyph_info.glyph_id as u16),
                            alpha: 1.0,
                            position: (data.world_position.x, data.world_position.y).into(),
                            matrix,
                        };
                        glyphs_to_draw.push((glyph_rect, item));

                        glyph_total_xadvance += x_advance;

                        glyph_index += 1;
                    }
                }

                prev = Some(current);
            }

            // render only completed text
            if glyph_index >= glyphs_len {
                let contains = self.id_to_alpha_map.contains_key(&data.id);
                let mut alpha = *self.id_to_alpha_map.entry(data.id).or_insert(data.alpha);
                if contains {
                    data.alpha = alpha;
                    return;
                }

                let rects = glyphs_to_draw
                    .iter()
                    .map(|(rect, _)| rect.clone())
                    .collect();
                if collision_handler.insert_rectangles(rects) {
                    alpha = clamp(alpha + Self::FADE_ANIM_SPEED, 0.0, 1.0);
                } else {
                    alpha = clamp(alpha - Self::FADE_ANIM_SPEED, 0.0, 1.0);
                };
                data.alpha = alpha;

                for (_, mut item) in glyphs_to_draw {
                    item.alpha = data.alpha;
                    self.glyph_data
                        .entry(item.glyph_id)
                        .and_modify(|list| {
                            list.push(item.clone());
                        })
                        .or_insert(vec![item.clone()]);
                }
            }

            return;
        }

        let origin = screen_position_calculator
            .screen_position(data.world_position.cast().unwrap())
            + coord! { x: data.screen_offset.x as f64, y: -data.screen_offset.y as f64}
            + coord! { x: (-width/2.0) as f64, y: 0.0 };

        let section_rect = Rectangle::from_corners(
            point! { x: origin.x as f32, y: origin.y as f32 },
            point! { x: origin.x as f32 + width, y: origin.y as f32 + height },
        );

        let within_screen = collision_handler.within_screen(config, section_rect);
        if within_screen {
            let contains = self.id_to_alpha_map.contains_key(&data.id);
            let mut alpha = *self.id_to_alpha_map.entry(data.id).or_insert(data.alpha);
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

                let matrix = Matrix4::from_translation(Vector3::new(
                    glyph_total_xadvance + data.screen_offset.x + (-width / 2.0),
                    -height + data.screen_offset.y,
                    0.0,
                )) * scale_m;
                let item = GlyphData {
                    glyph_id: GlyphId(glyph_info.glyph_id as u16),
                    alpha: data.alpha,
                    position: (data.world_position.x, data.world_position.y).into(),
                    matrix,
                };
                self.glyph_data
                    .entry(item.glyph_id)
                    .and_modify(|list| {
                        list.push(item.clone());
                    })
                    .or_insert(vec![item.clone()]);

                glyph_total_xadvance += position.x_advance as f32 * scale;
            }
        }
    }

    fn update_attrs(&mut self, device: &Device) {
        self.instance_buffer_map.clear();
        self.glyph_data.iter().for_each(|(key, list)| {
            let mut attrs = vec![];
            list.iter().for_each(|glyph_data| {
                let instance_pos = InstancePos {
                    position: Vector3::new(glyph_data.position.0, glyph_data.position.1, 0.0)
                        .into(),
                    color_alpha: glyph_data.alpha,
                    matrix: glyph_data.matrix.cast().unwrap().into(),
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

    pub fn render(&mut self, device: &wgpu::Device, render_pass: &mut RenderPass) {
        self.id_to_alpha_map.clear();

        self.update_attrs(device);

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
