use crate::collider::{ColliderTask, CollisionTaskController, CollisionTaskWrapper};
use crate::collision_handler::CollisionHandler;
use crate::geometry_data::TextData;
use crate::global_context::GlobalContext;
use crate::mesh::InstanceBuffer;
use crate::mesh_layers::render_data_holder::RenderDataHolder;
use crate::text::default_face_wrapper::DefaultFaceWrapper;
use crate::vertex_attrs::TextInstanceInput;
use crate::view_projection::ViewProjection;
use geo_types::{coord, point};
use rstar::primitives::Rectangle;
use rustc_hash::FxHashMap;
use rustybuzz::ttf_parser::GlyphId;
use std::collections::HashMap;
use std::sync::Arc;
use glam::{vec3, DVec3, Mat4, Quat, Vec3};
use wgpu::RenderPass;

#[derive(Clone)]
pub struct GlyphData {
    pub glyph_id: GlyphId,
    pub position: (f32, f32),
    pub alpha: f32,
    pub matrix: Mat4,
    pub screen_space: bool,
}

pub struct TextRenderer {
    default_face: Arc<DefaultFaceWrapper>,
    collision_task_controller: CollisionTaskController<TextData, FxHashMap<GlyphId, Vec<GlyphData>>>,
    instance_buffer_map: FxHashMap<GlyphId, InstanceBuffer<TextInstanceInput>>,
}

impl TextRenderer {
    pub fn new(
        global_context: &mut GlobalContext,
        font: &'static rustybuzz::ttf_parser::Face,
    ) -> TextRenderer {
        let device = global_context.device();
        let default_face = Arc::new(DefaultFaceWrapper::new(device, font));
        let (task_wrapper, collision_task_controller) = CollisionTaskWrapper::new();
        
        let task = TextRendererCollisionHandler::new(Arc::clone(&default_face), task_wrapper);
        global_context.collider.register_task(Box::new(task));
        TextRenderer {
            default_face,
            collision_task_controller,
            instance_buffer_map: FxHashMap::default(),
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
                let mut position = Vec3::new(glyph_data.position.0, glyph_data.position.1, 0.0);
                if !glyph_data.screen_space {
                    position -= vec3(cs_offset.x as f32, cs_offset.y as f32, 0.0)
                }
                let instance_input = TextInstanceInput {
                    position: position.into(),
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
            glyph_data.iter().for_each(|(glyph_id, list)| {
                if let Some(mesh) = self.default_face.glyph_mesh_map.get(glyph_id) {
                    let v_buf = &mesh.vertex_buf;
                    let (i_buf, i_buf_len) = &mesh.index_buf;
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
    task_wrapper: CollisionTaskWrapper<TextData, FxHashMap<GlyphId, Vec<GlyphData>>>
}

impl TextRendererCollisionHandler {
    const FADE_ANIM_SPEED: f32 = 0.05;
    pub fn new(
        default_face: Arc<DefaultFaceWrapper>,
        task_wrapper: CollisionTaskWrapper<TextData, FxHashMap<GlyphId, Vec<GlyphData>>>
    ) -> Self {
        TextRendererCollisionHandler {
            id_to_alpha_map: HashMap::new(),
            default_face,
            task_wrapper
        }
    }
}

impl ColliderTask for TextRendererCollisionHandler {
    fn run(&mut self, view_projection: &ViewProjection, collision_handler: &mut CollisionHandler) {
        let render_data_holder = self.task_wrapper.update_holder();
        

        let mut glyph_data: FxHashMap<GlyphId, Vec<GlyphData>> = FxHashMap::default();
        render_data_holder.run_mut_action(|data| {
            let glyph_buffer = data
                .glyph_buffer
                .get_or_insert_with(|| self.default_face.shape(data.text.as_str()));

            let glyphs_positions = glyph_buffer.glyph_positions();
            let glyphs_infos = glyph_buffer.glyph_infos();

            let (scale_m, width, height, scale) =
                self.default_face.get_text_params(&glyph_buffer, data.size);

            let mut glyphs_to_draw = vec![];

            let middle_point_index = data.positions.len() / 2;
            let initial_position: DVec3 = *data
                .positions
                .get(middle_point_index)
                .unwrap();
            let origin = view_projection.screen_position(&initial_position)
                + coord! { x: data.screen_offset.x as f64, y: data.screen_offset.y as f64};

            if data.positions.len() > 1 {
                let line_positions = &data.positions;

                let middle_point_index = line_positions.len() / 2;
                let mut prev: Option<Vec3> = None;
                let mut glyph_index = 0;
                let glyphs_len = glyph_buffer.len();
                let segments_count = line_positions.len();

                let mut segments_len = 0.0;
                let mut segments_vector = Vec3::new(0.0, 0.0, 0.0);

                let mut backward = false;

                let flip_rot_m = Mat4::from_axis_angle(Vec3::Z, 180.0f32.to_radians());
                let half_height_translation =
                    Mat4::from_translation(Vec3::new(0.0, -height / 2.0, 0.0));

                for (index, current) in line_positions[middle_point_index..].iter().enumerate() {
                    if glyph_index >= glyphs_len {
                        break;
                    }

                    let current = view_projection.screen_position(&current) - origin;
                    let current = Vec3::new(current.x as f32, current.y as f32, 0.0);

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
                        segments_len += seg_vector.length();


                        let seg_rotation: Quat =
                            Quat::from_rotation_arc(
                                seg_vector.normalize(),
                                Vec3::X
                            );


                        let rot_m: Mat4 = Mat4::from_quat(seg_rotation);
                        let scale_rot_height_m = scale_m * rot_m * half_height_translation;

                        while glyph_index < glyphs_len {
                            if index < segments_count - 1 && segments_vector.length() > segments_len
                            {
                                break;
                            }

                            let real_glyph_index = if backward {
                                glyphs_len - glyph_index - 1
                            } else {
                                glyph_index
                            };

                            let position = glyphs_positions[real_glyph_index];
                            let glyph_info = glyphs_infos[real_glyph_index];

                            let x_advance = position.x_advance as f32 * scale;
                            let x_advance_vector = Vec3::new(x_advance, 0.0, 0.0);
                            let rotated_glyph_vector = seg_rotation * x_advance_vector;

                            let matrix = if backward {
                                let x_advance_translation =
                                    Mat4::from_translation(-x_advance_vector);
                                Mat4::from_translation(segments_vector)
                                    * flip_rot_m
                                    * scale_rot_height_m
                                    * x_advance_translation
                            } else {
                                Mat4::from_translation(segments_vector) * scale_rot_height_m
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
                                screen_space: data.screen_space,
                            };
                            glyphs_to_draw.push((glyph_rect, item));

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
                let origin = origin + coord! { x: (-width/2.0) as f64, y: 0.0 };

                let mut glyph_total_x_advance = 0.0;

                let section_rect = Rectangle::from_corners(
                    point! { x: origin.x as f32, y: origin.y as f32 },
                    point! { x: origin.x as f32 + width, y: origin.y as f32 + height },
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
                        for index in 0..glyph_buffer.len() {
                            let position = glyphs_positions[index];
                            let glyph_info = glyphs_infos[index];
                            let matrix = Mat4::from_translation(Vec3::new(
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
