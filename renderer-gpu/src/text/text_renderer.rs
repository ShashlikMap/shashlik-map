use crate::buffer_pool::BufferPool;
use crate::collider::{ColliderTask, CollisionTaskController, CollisionTaskWrapper};
use crate::global_context::GlobalContext;
use crate::mesh::InstanceBuffer;
use crate::mesh::mesh_instance_input::{MeshInstanceInput};
use crate::mesh_layers::render_data_holder::RenderDataHolder;
use crate::text::default_face_wrapper::DefaultFaceWrapper;
use crate::text::glyph_cache::GlyphCache;
use crate::view_projection::ViewProjection;
use geo_types::{coord, point};
use glam::{DVec3, Mat4, Vec2, Vec3, dvec3, vec3};
use num::clamp;
use renderer_common::collision_handler::CollisionHandler;
use renderer_common::geometry_data::TextData;
use rstar::primitives::Rectangle;
use rustc_hash::FxHashMap;
use rustybuzz::ttf_parser::GlyphId;
use splines::{Interpolation, Key, Spline};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::mem;
use std::ops::Range;
use std::sync::Arc;
use log::error;
use wgpu::RenderPass;
use crate::mesh_layers::{LayerAttrMapper, LayerAttribute};

#[derive(Clone)]
pub struct GlyphData {
    pub glyph_id: GlyphId,
    pub position: (f64, f64),
    pub alpha: f32,
    pub matrix: Mat4,
    pub screen_space: bool,
}

pub struct TextRenderer<I: MeshInstanceInput> {
    attr_map: LayerAttrMapper<I>,
    collision_task_controller:
        CollisionTaskController<TextData, FxHashMap<GlyphId, Vec<GlyphData>>>,
    instance_buffer_ranges: Vec<Range<u32>>,
    instance_buffer: InstanceBuffer<I>,
    glyph_cache: GlyphCache,
    glyph_data: FxHashMap<GlyphId, Vec<GlyphData>>,
    buffer_pool: BufferPool // Just a convenient stub to create buffers for text
}

impl<I: MeshInstanceInput> TextRenderer<I> {
    pub fn new(
        global_context: &mut GlobalContext,
        font: rustybuzz::ttf_parser::Face<'static>,
        attr_map: LayerAttrMapper<I>
    ) -> Self {
        let default_face = Arc::new(DefaultFaceWrapper::new(font));
        let (task_wrapper, collision_task_controller) = CollisionTaskWrapper::new();

        let glyph_cache = GlyphCache::new(Arc::clone(&default_face));
        let task = TextRendererCollisionHandler::new(Arc::clone(&default_face), task_wrapper);
        global_context.collider.register_task(Box::new(task));
        Self {
            attr_map,
            collision_task_controller,
            instance_buffer_ranges: Vec::new(),
            instance_buffer: InstanceBuffer::default(),
            glyph_cache,
            glyph_data: FxHashMap::default(),
            buffer_pool: BufferPool::new()
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
        let cs_offset = dvec3(cs_offset.x, cs_offset.y, 0.0);
        let total_len = glyph_data.iter().map(|it| it.1.len()).sum::<usize>();
        let mut attrs = Vec::with_capacity(total_len);
        self.instance_buffer_ranges.clear();
        glyph_data.iter().for_each(|(_, list)| {
            let start_index = attrs.len() as u32;

            list.iter().for_each(|glyph_data| {
                let mut position = DVec3::new(glyph_data.position.0, glyph_data.position.1, 0.0);
                if !glyph_data.screen_space {
                    position -= cs_offset;
                }

                let instance_input = (self.attr_map)(LayerAttribute {
                    position: position.as_vec3().into(),
                    color_alpha: glyph_data.alpha,
                    matrix: glyph_data.matrix.to_cols_array_2d(),
                    screen_space: glyph_data.screen_space.into(),
                    ..Default::default()
                });
                attrs.push(instance_input);
            });

            let end_index = attrs.len() as u32;
            self.instance_buffer_ranges.push(start_index..end_index);
        });

        self.instance_buffer.update("TextInstanceBuffer", global_context, &attrs);
    }

    pub fn update(&mut self, global_context: &mut GlobalContext) {
        let Ok(glyph_data) = self.collision_task_controller.receiver.try_recv() else {
            return;
        };
        self.update_attrs(global_context, &glyph_data);
        self.glyph_data = glyph_data;
    }

    pub fn render(&mut self, render_pass: &mut RenderPass, global_context: &GlobalContext) {
        let glyph_data = mem::take(&mut self.glyph_data);

        if !self.instance_buffer_ranges.is_empty() && !glyph_data.is_empty() {
            self.glyph_cache.process_glyph_data(global_context, &mut self.buffer_pool,
                                                glyph_data, |mesh, index_ranges| {
                    let v_buf = &mesh.vertex_buf;
                    if v_buf.size() > 0 {
                        let (i_buf, _) = &mesh.index_buf;
                        render_pass.set_vertex_buffer(0, v_buf.slice(..));
                        render_pass.set_index_buffer(i_buf.slice(..), wgpu::IndexFormat::Uint32);
                        if let Some(instance_buffer) = self.instance_buffer.buffer_with_id.as_ref() {
                            render_pass.set_vertex_buffer(1, instance_buffer.buffer().slice(..));

                            if self.instance_buffer_ranges.len() != index_ranges.len() {
                                error!("Glyph instance and indices ranges length are not equal");
                            } else {
                                index_ranges.into_iter().zip(self.instance_buffer_ranges.iter()).for_each(|(index_range, instance_range)| {
                                    render_pass.draw_indexed(index_range, 0, instance_range.clone());
                                })
                            }
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
        let tangent_basis = Vec2::X;

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

                let mut skip_process = false;
                // skip if the text is about to exceed the line and the text already invisible
                if data.alpha == 0.0 {
                    skip_process = total_length - face_text_params.width * 0.5 < 0.0;
                    if !skip_process {
                        let mmm = collision_handler.point_within_screen(projected.first().unwrap())
                            || collision_handler.point_within_screen(projected.last().unwrap());
                        skip_process = !mmm;
                    }
                }

                if !skip_process {
                    let origin = projected[index_of_center_segment] + projected_segments[index_of_center_segment].normalize_or_zero() * length_remainder;
                    let unprojected_origin = view_projection.screen_to_world(&origin).unwrap();

                    let origin_vec = vec![origin];
                    let new_list = origin_vec.iter().chain(projected[(index_of_center_segment + 1)..].iter());

                    let mut prev_tangent: Option<Vec2> = None;
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
                        if !backward && item.x < previous_spline_segment.x && previous_spline_segment == &origin {
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

                            let seg_rot = tangent.angle_to(tangent_basis);

                            if let Some(prev_tangent) = prev_tangent {
                                if tangent.angle_to(prev_tangent).to_degrees().abs() >= Self::SHARP_ANGLE_THRESHOLD {
                                    discard_animated = true;
                                    break;
                                }
                            }
                            prev_tangent = Some(tangent);

                            let rot_m: Mat4 = Mat4::from_rotation_z(seg_rot);
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
                        if collision_handler.check_and_insert_rectangles(rects) {
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
                        if collision_handler.check_and_insert(section_rect) {
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
                if data.alpha > 0.0 {
                    glyph_data
                        .entry(item.glyph_id)
                        .and_modify(|list| {
                            list.push(item.clone());
                        })
                        .or_insert(vec![item]);
                }
            }
        });

        self.id_to_alpha_map.clear();
        self.task_wrapper.send_result(glyph_data);
    }
}
