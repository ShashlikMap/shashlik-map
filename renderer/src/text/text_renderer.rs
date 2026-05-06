use crate::collider::{ColliderTask, CollisionTaskController, CollisionTaskWrapper};
use crate::collision_handler::CollisionHandler;
use crate::geometry_data::TextData;
use crate::global_context::GlobalContext;
use crate::mesh::InstanceBuffer;
use crate::mesh_layers::render_data_holder::RenderDataHolder;
use crate::text::default_face_wrapper::DefaultFaceWrapper;
use crate::text::glyph_cache::GlyphCache;
use crate::vertex_attrs::TextInstanceInput;
use crate::view_projection::ViewProjection;
use geo_types::{coord, point};
use glam::{dvec3, vec3, DVec3, Mat4, Quat, Vec2, Vec3};
use num::clamp;
use rstar::primitives::Rectangle;
use rustc_hash::FxHashMap;
use rustybuzz::ttf_parser::GlyphId;
use splines::{Interpolation, Key, Spline};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::Arc;
use wgpu::RenderPass;

#[derive(Clone)]
pub struct GlyphData {
    pub glyph_id: GlyphId,
    pub position: (f64, f64),
    pub alpha: f32,
    pub matrix: Mat4,
    pub screen_space: bool,
}

pub struct TextRenderer {
    collision_task_controller:
        CollisionTaskController<TextData, FxHashMap<GlyphId, Vec<GlyphData>>>,
    instance_buffer_map: FxHashMap<GlyphId, InstanceBuffer<TextInstanceInput>>,
    glyph_cache: GlyphCache
}

impl TextRenderer {
    pub fn new(
        global_context: &mut GlobalContext,
        font: &'static rustybuzz::ttf_parser::Face,
    ) -> TextRenderer {
        let default_face = Arc::new(DefaultFaceWrapper::new(font));
        let (task_wrapper, collision_task_controller) = CollisionTaskWrapper::new();

        let glyph_cache = GlyphCache::new(Arc::clone(&default_face));
        let task = TextRendererCollisionHandler::new(Arc::clone(&default_face), task_wrapper);
        global_context.collider.register_task(Box::new(task));
        TextRenderer {
            collision_task_controller,
            instance_buffer_map: FxHashMap::default(),
            glyph_cache,
        }
    }

    pub fn update_data<F: FnOnce(&mut RenderDataHolder<TextData>) + Send + 'static>(
        &self,
        updater: F,
    ) {
        self.collision_task_controller.update_data(updater);
    }

    fn update_attrs(
        &mut self,
        global_context: &GlobalContext,
        glyph_data: &FxHashMap<GlyphId, Vec<GlyphData>>,
    ) {
        let cs_offset = global_context.view_projection.cs_offset;
        let device = global_context.device();
        let queue = global_context.queue();
        glyph_data.iter().for_each(|(key, list)| {
            let mut attrs = vec![];
            list.iter().for_each(|glyph_data| {
                let mut position = DVec3::new(glyph_data.position.0, glyph_data.position.1, 0.0);
                if !glyph_data.screen_space {
                    position -= dvec3(cs_offset.x, cs_offset.y, 0.0)
                }
                let instance_input = TextInstanceInput {
                    position: position.as_vec3().into(),
                    color_alpha: glyph_data.alpha,
                    matrix: glyph_data.matrix.to_cols_array_2d(),
                    screen_space: glyph_data.screen_space.into(),
                };
                attrs.push(instance_input);
            });

            let instance_buffer = self
                .instance_buffer_map
                .entry(*key)
                .or_insert(InstanceBuffer::default());
            instance_buffer.update("TextInstanceBuffer", device, queue, &attrs);
        });
    }

    pub fn render(&mut self, render_pass: &mut RenderPass, global_context: &GlobalContext) {
        let Ok(glyph_data) = self.collision_task_controller.receiver.try_recv() else {
            return;
        };

        self.update_attrs(global_context, &glyph_data);

        if !self.instance_buffer_map.is_empty() && !glyph_data.is_empty() {
            let device = global_context.device();
            glyph_data.iter().for_each(|(glyph_id, list)| {
                let glyph_mesh = self.glyph_cache.get_or_tessellate(device, glyph_id);
                let v_buf = &glyph_mesh.vertex_buf;
                if v_buf.size() > 0 {
                    let (i_buf, i_buf_len) = &glyph_mesh.index_buf;
                    let instance_buffer = self.instance_buffer_map.get(glyph_id).unwrap();
                    if let Some(instance_buffer) = instance_buffer.buffer.as_ref() {
                        render_pass.set_vertex_buffer(0, v_buf.slice(..));
                        render_pass.set_index_buffer(i_buf.slice(..), wgpu::IndexFormat::Uint32);

                        render_pass.set_vertex_buffer(1, instance_buffer.slice(..));

                        render_pass.draw_indexed(0..*i_buf_len as u32, 0, 0..list.len() as u32);
                    }
                }
            });
        }
    }
}

struct TextRendererCollisionHandler {
    id_to_alpha_map: HashMap<u64, f32>,
    default_face: Arc<DefaultFaceWrapper>,
    task_wrapper: CollisionTaskWrapper<TextData, FxHashMap<GlyphId, Vec<GlyphData>>>,
}

impl TextRendererCollisionHandler {
    const FADE_ANIM_SPEED: f32 = 0.05;
    const SHARP_ANGLE_THRESHOLD: f32 = 30.0;
    const SPLINE_TANGENT_OFFSET: f32 = 2.0;
    pub fn new(
        default_face: Arc<DefaultFaceWrapper>,
        task_wrapper: CollisionTaskWrapper<TextData, FxHashMap<GlyphId, Vec<GlyphData>>>,
    ) -> Self {
        TextRendererCollisionHandler {
            id_to_alpha_map: HashMap::new(),
            default_face,
            task_wrapper,
        }
    }
}

impl ColliderTask for TextRendererCollisionHandler {
    fn run(&mut self, view_projection: &ViewProjection, collision_handler: &mut CollisionHandler)  {
        let render_data_holder = self.task_wrapper.update_holder();

        let flip_rot_m = Mat4::from_rotation_z(PI);

        let mut glyph_data: FxHashMap<GlyphId, Vec<GlyphData>> = FxHashMap::default();
        render_data_holder.run_mut_action(|data| {
            let glyph_buffer = data.glyph_buffer
                .get_or_insert_with(|| self.default_face.shape(data.text.as_str()));

            let glyphs_positions = glyph_buffer.glyph_positions();
            let glyphs_infos = glyph_buffer.glyph_infos();

            let glyphs_len = glyph_buffer.len();

            let face_text_params = data.face_text_params
                .get_or_insert_with(|| self.default_face.get_text_params(&glyph_buffer, data.size));

            let mut glyphs_to_draw = vec![];

            if data.line_data.positions.len() > 1 {
                let mut index_of_center_segment = data.line_data.get_center_segment_index();

                let projected: Vec<_> = data.line_data.positions.iter()
                    .map(|&p| {
                        let c = view_projection.screen_position(&p);
                        Vec2::new(c.x as f32, c.y as f32)
                    })
                    .collect();

                let projected_segments: Vec<_> = projected
                    .windows(2)
                    .map(|pair| pair[1] - pair[0]).collect();

                let mut total_length = projected_segments[index_of_center_segment].length() * 0.5;
                while index_of_center_segment > 0 && total_length < (face_text_params.width * 0.5) {
                    index_of_center_segment -= 1;
                    total_length += projected_segments[index_of_center_segment].length();
                };
                let length_remainder = total_length - face_text_params.width * 0.5;
                let origin = projected[index_of_center_segment] + projected_segments[index_of_center_segment].normalize_or_zero() * length_remainder;
                let unprojected_origin = view_projection.screen_to_world(&origin).unwrap();

                let origin_vec = vec![origin];
                let new_list = origin_vec.iter().chain(projected[(index_of_center_segment + 1)..].iter());

                let mut prev_angle_rad: Option<Quat> = None;
                let mut glyph_index = 0;

                let mut segments_vector_length: f32 = 0.0;

                let mut backward = false;

                let mut discard_animated = false;

                let mut previous_spline_segment = &origin;
                let mut accum_length = 0.0;

                // there should be an extra "ghost" origin_vec since CatmullRom requires more points
                let spline = Spline::from_iter(origin_vec.iter().chain(new_list).map(|item| {
                    let current_length = item - origin;
                    accum_length += (item - previous_spline_segment).length();
                    if !backward && item.x < previous_spline_segment.x && previous_spline_segment == &origin  {
                        backward = true;
                    }
                    previous_spline_segment = item;
                    Key::new(accum_length, current_length, Interpolation::CatmullRom)
                }));

                while glyph_index < glyphs_len {
                    if !segments_vector_length.is_nan() && let (Some(spline_position), Some(spline_position_offset))
                        = (spline.sample(segments_vector_length), spline.sample(segments_vector_length + Self::SPLINE_TANGENT_OFFSET)) {
                        let real_glyph_index = if backward {
                            glyphs_len - glyph_index - 1
                        } else {
                            glyph_index
                        };

                        let position = glyphs_positions[real_glyph_index];
                        let glyph_info = glyphs_infos[real_glyph_index];

                        let tangent = (spline_position_offset - spline_position).normalize();

                        let seg_rotation: Quat =
                            Quat::from_rotation_arc(
                                vec3(tangent.x, tangent.y, 0.0),
                                Vec3::X,
                            );

                        if let Some(prev_angle_rad) = prev_angle_rad {
                            if seg_rotation.angle_between(prev_angle_rad).to_degrees() >= Self::SHARP_ANGLE_THRESHOLD {
                                discard_animated = true;
                                break;
                            }
                        }
                        prev_angle_rad = Some(seg_rotation);

                        let rot_m: Mat4 = Mat4::from_quat(seg_rotation);
                        let scale_rot_height_m = face_text_params.scale_matrix * rot_m * face_text_params.half_height_translation;

                        let x_advance = position.x_advance as f32 * face_text_params.scale;

                        let spline_pos_translation = Mat4::from_translation(vec3(spline_position.x, -spline_position.y, 0.0));
                        let matrix = if backward {
                            // FIXME 1.5 somehow depends on the font size
                            let x_advance_translation = Mat4::from_translation(-Vec3::new(1.5 * x_advance, 0.0, 0.0));
                            spline_pos_translation * flip_rot_m * scale_rot_height_m * x_advance_translation
                        } else {
                            spline_pos_translation * scale_rot_height_m
                        };

                        segments_vector_length += x_advance;

                        // note: segments_vector.y goes negative so we should diff y-axis!
                        let height = face_text_params.height;
                        let glyph_rect = Rectangle::from_corners(
                            point! { x: origin.x + spline_position.x - height, y: origin.y + spline_position.y - height },
                            point! { x: origin.x + spline_position.x + height, y: origin.y + spline_position.y + height},
                        );

                        let item = GlyphData {
                            glyph_id: GlyphId(glyph_info.glyph_id as u16),
                            alpha: 1.0,
                            position: (unprojected_origin.x, unprojected_origin.y),
                            matrix,
                            screen_space: data.screen_space,
                        };
                        glyphs_to_draw.push((glyph_rect, item));

                        glyph_index += 1;
                    } else {
                        discard_animated = true;
                        break;
                    }
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
                } else if discard_animated {
                    data.alpha = clamp(data.alpha - Self::FADE_ANIM_SPEED, 0.0, 1.0);
                } else {
                    glyphs_to_draw.clear();
                }
            } else {
                let middle_point_index = data.line_data.positions.len() / 2;
                let initial_position: DVec3 = *data.line_data
                    .positions
                    .get(middle_point_index)
                    .unwrap();
                let origin = view_projection.screen_position(&initial_position)
                    + coord! { x: data.screen_offset.x as f64, y: data.screen_offset.y as f64};

                let origin = origin + coord! { x: (-face_text_params.width/2.0) as f64, y: 0.0 };

                let mut glyph_total_x_advance = 0.0;

                let section_rect = Rectangle::from_corners(
                    point! { x: origin.x as f32, y: origin.y as f32 },
                    point! { x: origin.x as f32 + face_text_params.width, y: origin.y as f32 + face_text_params.height },
                );

                let within_screen = collision_handler.within_screen(section_rect);
                if data.screen_space || within_screen {
                    let contains = self.id_to_alpha_map.contains_key(&data.id);
                    let mut alpha = *self.id_to_alpha_map.entry(data.id).or_insert(data.alpha);
                    if contains {
                        data.alpha = alpha;
                        return;
                    }

                    // calc only for non screen space
                    if !data.screen_space {
                        if collision_handler.insert(section_rect) {
                            alpha = clamp(alpha + Self::FADE_ANIM_SPEED, 0.0, 1.0);
                        } else {
                            alpha = clamp(alpha - Self::FADE_ANIM_SPEED, 0.0, 1.0);
                        }
                    }
                    data.alpha = alpha;
                    if data.alpha > 0.0 {
                        let stub_rect =
                            Rectangle::from_corners(point!(x: 0.0, y: 0.0), point!(x: 0.0, y: 0.0));
                        for index in 0..glyphs_len {
                            let position = glyphs_positions[index];
                            let glyph_info = glyphs_infos[index];
                            let matrix = Mat4::from_translation(Vec3::new(
                                glyph_total_x_advance + data.screen_offset.x + (-face_text_params.width / 2.0),
                                -face_text_params.height - data.screen_offset.y,
                                0.0,
                            )) * face_text_params.scale_matrix;

                            glyph_total_x_advance += position.x_advance as f32 * face_text_params.scale;

                            let item = GlyphData {
                                glyph_id: GlyphId(glyph_info.glyph_id as u16),
                                alpha: data.alpha,
                                position: (initial_position.x, initial_position.y),
                                matrix,
                                screen_space: data.screen_space,
                            };
                            glyphs_to_draw.push((stub_rect, item));
                        }
                    }
                }
            }

            for (_, mut item) in glyphs_to_draw {
                item.alpha = data.alpha;
                glyph_data
                    .entry(item.glyph_id)
                    .and_modify(|list| {
                        if data.alpha > 0.0 {
                            list.push(item.clone());
                        }
                    })
                    .or_insert(vec![item.clone()]);
            }
        });

        self.id_to_alpha_map.clear();
        self.task_wrapper.send_result(glyph_data);
    }
}
