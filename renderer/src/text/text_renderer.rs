use crate::camera::ScreenPositionCalculator;
use crate::collision_handler::CollisionHandler;
use crate::text::default_face_wrapper::DefaultFaceWrapper;
use crate::vertex_attrs::InstancePos;
use cgmath::num_traits::clamp;
use cgmath::{Deg, InnerSpace, Matrix4, Quaternion, Rotation, Vector2, Vector3};
use geo_types::{Coord, coord, point};
use log::error;
use rstar::primitives::Rectangle;
use rustc_hash::FxHashMap;
use rustybuzz::GlyphBuffer;
use rustybuzz::ttf_parser::GlyphId;
use std::alloc::System;
use std::collections::HashMap;
use std::time::SystemTime;
use wgpu::util::DeviceExt;
use wgpu::{Buffer, Device, Queue, RenderPass};

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
    pub positions: Vec<Vector3<f32>>,
    pub screen_offset: Vector2<f32>,
    pub glyph_buffer: Option<GlyphBuffer>,
}

pub struct TextRenderer {
    id_to_alpha_map: HashMap<u64, f32>,
    default_face: DefaultFaceWrapper,
    glyph_data: FxHashMap<GlyphId, Vec<GlyphData>>,
    instance_buffer_map: FxHashMap<GlyphId, (usize, Buffer)>,
}

impl TextRenderer {
    const FADE_ANIM_SPEED: f32 = 0.05;

    pub fn new(device: &Device) -> TextRenderer {
        let default_face = DefaultFaceWrapper::new(device);

        TextRenderer {
            id_to_alpha_map: HashMap::new(),
            default_face,
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
        let glyph_buffer = data
            .glyph_buffer
            .get_or_insert_with(|| self.default_face.shape(data.text.as_str()));

        let glyphs_positions = glyph_buffer.glyph_positions();
        let glyphs_infos = glyph_buffer.glyph_infos();
        let mut glyph_total_x_advance = 0.0;

        let (scale_m, width, height, scale) =
            self.default_face.get_text_params(&glyph_buffer, data.size);

        let mut glyphs_to_draw = vec![];

        let middle_point_index = data.positions.len() / 2;
        let initial_position: Vector3<f64> = data
            .positions
            .get(middle_point_index)
            .unwrap()
            .cast()
            .unwrap();

        if data.positions.len() > 1 {
            let line_positions = &data.positions;

            let origin = screen_position_calculator.screen_position(initial_position);

            let middle_point_index = line_positions.len() / 2;
            let mut prev: Option<Coord<f32>> = None;
            let mut glyph_index = 0;
            let glyphs_len = glyph_buffer.len();
            let segments_count = line_positions.len();

            let mut segments_len = 0.0;
            let mut segments_vector = Vector3::new(0.0, 0.0, 0.0);

            let mut backward = false;

            let flip_rot_m = Matrix4::from_angle_z(Deg(180.0));

            for (index, current) in line_positions[middle_point_index..].iter().enumerate() {
                if glyph_index >= glyphs_len {
                    break;
                }

                let current =
                    screen_position_calculator.screen_position(current.cast().unwrap()) - origin;
                let current = coord! {x : current.x as f32, y: current.y as f32 };

                // skip if two point are the same
                if let Some(prev) = prev
                    && prev != current
                {
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

                        // note: segments_vector.y goes negative so we should diff y-axis!
                        let glyph_rect = Rectangle::from_corners(
                            point! { x: origin.x as f32 + segments_vector.x - height, y: origin.y as f32 - segments_vector.y - height },
                            point! { x: origin.x as f32 + segments_vector.x + height, y: origin.y as f32 - segments_vector.y + height},
                        );

                        segments_vector += rotated_glyph_vector;

                        let item = GlyphData {
                            glyph_id: GlyphId(glyph_info.glyph_id as u16),
                            alpha: 1.0,
                            position: (initial_position.x as f32, initial_position.y as f32),
                            matrix,
                        };
                        glyphs_to_draw.push((glyph_rect, item));

                        glyph_total_x_advance += x_advance;

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
            } else {
                glyphs_to_draw.clear();
            }
        } else {
            let origin = screen_position_calculator.screen_position(initial_position)
                + coord! { x: data.screen_offset.x as f64, y: data.screen_offset.y as f64}
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

                if data.alpha > 0.0 {
                    for index in 0..glyph_buffer.len() {
                        let position = glyphs_positions[index];
                        let glyph_info = glyphs_infos[index];

                        let matrix = Matrix4::from_translation(Vector3::new(
                            glyph_total_x_advance + data.screen_offset.x + (-width / 2.0),
                            -height - data.screen_offset.y,
                            0.0,
                        )) * scale_m;

                        glyph_total_x_advance += position.x_advance as f32 * scale;

                        let item = GlyphData {
                            glyph_id: GlyphId(glyph_info.glyph_id as u16),
                            alpha: data.alpha,
                            position: (initial_position.x as f32, initial_position.y as f32),
                            matrix,
                        };
                        glyphs_to_draw.push((
                            Rectangle::from_corners(point!(x: 0.0, y: 0.0), point!(x: 0.0, y: 0.0)),
                            item,
                        ));
                    }
                }
            }
        }

        for (_, mut item) in glyphs_to_draw {
            item.alpha = data.alpha;
            self.glyph_data
                .entry(item.glyph_id)
                .and_modify(|list| {
                    if data.alpha > 0.0 {
                        list.push(item.clone());
                    }
                })
                .or_insert(vec![item.clone()]);
        }
    }

    fn update_attrs(&mut self, queue: &Queue, device: &Device) {
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

            let attrs_len = attrs.len();
            self.instance_buffer_map
                .entry(*key)
                .and_modify(|list| {
                    if list.0 < attrs_len {
                        list.1 = Self::create_instance_buffer(device, &attrs);
                    } else {
                        queue.write_buffer(&list.1, 0, bytemuck::cast_slice(attrs.as_slice()));
                    }
                    list.0 = attrs_len;
                })
                .or_insert_with(|| {
                    let instance_buffer = Self::create_instance_buffer(device, &attrs);
                    (attrs_len, instance_buffer)
                });
        });
    }

    fn create_instance_buffer(device: &Device, instances: &Vec<InstancePos>) -> Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(instances.as_slice()),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        })
    }

    pub fn render(&mut self, queue: &Queue, device: &wgpu::Device, render_pass: &mut RenderPass) {
        self.id_to_alpha_map.clear();

        self.update_attrs(queue, device);

        if !self.instance_buffer_map.is_empty() && !self.glyph_data.is_empty() {
            self.glyph_data.iter().for_each(|(glyph_id, list)| {
                if let Some(mesh) = self.default_face.glyph_mesh_map.get(glyph_id) {
                    let v_buf = mesh.vertex_buf.get(0).unwrap();
                    let i_buf = mesh.index_buf.get(0).unwrap();
                    let instance_buffer = self.instance_buffer_map.get(glyph_id).unwrap();

                    render_pass.set_vertex_buffer(0, v_buf.slice(..));
                    render_pass.set_index_buffer(i_buf.0.slice(..), wgpu::IndexFormat::Uint32);

                    render_pass.set_vertex_buffer(1, instance_buffer.1.slice(..));

                    render_pass.draw_indexed(0..i_buf.1 as u32, 0, 0..list.len() as u32);
                }
            });
        }

        self.glyph_data.clear();
    }
}
