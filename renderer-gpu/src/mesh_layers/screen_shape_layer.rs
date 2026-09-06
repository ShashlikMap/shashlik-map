use crate::buffer_pool::BufferPool;
use crate::collider::{ColliderTask, CollisionTaskController, CollisionTaskWrapper};
use crate::draw_commands::mesh2d_draw_command::Mesh2dDrawCommand;
use crate::global_context::GlobalContext;
use crate::mesh::InstanceBuffer;
use crate::mesh::mesh::Mesh;
use crate::mesh::mesh_instance_input::{MeshInstanceInput};
use crate::mesh_layers::{BaseMeshLayer, LayerAttrMapper, RenderableLayer};
use crate::pipelines::RenderPipeline;
use crate::view_projection::ViewProjection;
use geo_types::point;
use glam::DVec3;
use num::clamp;
use renderer_common::collision_handler::CollisionHandler;
use renderer_common::render_modifier::SpatialData;
use rstar::primitives::Rectangle;
use std::collections::HashMap;
use std::mem;
use std::ops::Range;
use rustc_hash::{FxHashMap, FxHashSet};
use wgpu::RenderPass;
use crate::mesh_buffers::MeshBuffers;

// TODO ScreenMeshLayer and GeneralMeshLayer could be combined somehow.
pub(crate) struct ScreenShapeLayer<I: MeshInstanceInput> {
    attr_map: LayerAttrMapper<I>,
    instance_buffer: InstanceBuffer<I>,
    meshes: HashMap<String, (Mesh, Range<u32>)>,
    collision_task_controller: CollisionTaskController<
        (ShapeInfo, f32, String),
        FxHashMap<String, Vec<(DVec3, f32)>>,
    >,
}

struct ShapeInfo {
    pub id: u64,
    pub position: DVec3,
    pub size: f32,
}

impl<I: MeshInstanceInput> ScreenShapeLayer<I> {
    pub fn new(global_context: &mut GlobalContext, attr_map: LayerAttrMapper<I>) -> Self {
        let (task_wrapper, collision_task_controller) = CollisionTaskWrapper::new();
        let task = ScreenMeshCollisionHandler::new(task_wrapper);
        global_context.collider.register_task(Box::new(task));
        ScreenShapeLayer {
            attr_map,
            instance_buffer: InstanceBuffer::default(),
            meshes: HashMap::new(),
            collision_task_controller,
        }
    }

    pub fn submit(
        &mut self,
        key: &str,
        spatial_data: SpatialData,
        global_context: &mut GlobalContext,
        buffer_pool: &mut BufferPool,
        command: &mut Mesh2dDrawCommand,
    ) {
        let mut data = vec![];
        command.batches.iter_mut().for_each(|batch| {
            let instance_key = batch.mesh_info.instance_key.to_string();

            self.meshes.entry(instance_key.clone()).or_insert({
                (
                    Mesh::create_layered(
                        None, // Icons cache itself here!
                        global_context,
                        buffer_pool,
                        &batch.mesh,
                        mem::take(&mut batch.layers_indices),
                    ),
                    0..0
                )
            });

            let instance_positions =
                mem::take(&mut batch.mesh_info.instance_positions).unwrap_or_default();

            let size = batch.mesh_info.size.unwrap_or_default();


            let batch_data: Vec<_> = instance_positions.into_iter().map(|item| {
                let shape_info = ShapeInfo {
                    id: item.0,
                    position: item.1 + spatial_data.transform,
                    size
                };
                (shape_info, 0.0f32, instance_key.clone())
            }).collect();

            data.extend(batch_data)
        });

        let key = key.to_string();
        self.collision_task_controller.update_data(move |holder| {
            holder.set(key, data)
        });
    }
}

impl<I: MeshInstanceInput> BaseMeshLayer for ScreenShapeLayer<I> {
    fn update(&mut self, global_context: &mut GlobalContext) {
        let Ok(hm) = self.collision_task_controller.receiver.try_recv() else {
            return;
        };
        let cs_offset = global_context.view_projection.cs_offset;
        let mut all_attrs = vec![];
        self.meshes
            .iter_mut()
            .for_each(|(key, (_, instance_range))| {
                *instance_range = 0..0; // reset range before updating so it won't be counted during rendering
                if let Some(pos_alpha) = hm.get(key) {
                    let start_index = all_attrs.len() as u32;
                    let mut attrs = Vec::with_capacity(pos_alpha.len());
                    I::fill_attrs(
                        &mut attrs,
                        self.attr_map,
                        &cs_offset,
                        pos_alpha,
                        &SpatialData::new(),
                    );
                    all_attrs.extend(attrs);
                    let end_index = all_attrs.len() as u32;
                    *instance_range = start_index..end_index;
                }
            });
        self.instance_buffer.update(
            "ScreenInstanceBuffer",
            global_context,
            &all_attrs,
        );
    }
    
    fn clear_by_key(&mut self, key: &str) {
        self.collision_task_controller.clear_by_key(key);
    }
}

impl<I: MeshInstanceInput> RenderableLayer<I> for ScreenShapeLayer<I> {
    fn render(&mut self, render_pass: &mut RenderPass, render_pipeline: &mut impl RenderPipeline<I>, global_context: &mut GlobalContext) {
        render_pipeline.setup_render(render_pass, global_context);

        let mesh_buffers = MeshBuffers::builder().with_instance_buffer(&self.instance_buffer);
        render_pipeline.setup_mesh_buffers(render_pass, &mesh_buffers);

        self.meshes.iter().for_each(|(_, (mesh, instance_range))| {
            mesh.render_instanced_with_range(render_pass, global_context, instance_range, false, None);
        });
    }
}

struct ScreenMeshCollisionHandler {
    collision_task_wrapper: CollisionTaskWrapper<
        (ShapeInfo, f32, String),
        FxHashMap<String, Vec<(DVec3, f32)>>,
    >,
    id_to_alpha: FxHashMap<u64, f32>
}

impl ScreenMeshCollisionHandler {
    const FADE_ANIM_SPEED: f32 = 0.05;
    pub fn new(
        collision_task_wrapper: CollisionTaskWrapper<
            (ShapeInfo, f32, String),
            FxHashMap<String, Vec<(DVec3, f32)>>,
        >,
    ) -> Self {
        ScreenMeshCollisionHandler {
            collision_task_wrapper,
            id_to_alpha: FxHashMap::default(),
        }
    }
}

impl ColliderTask for ScreenMeshCollisionHandler {
    fn run(&mut self, view_projection: &ViewProjection, collision_handler: &mut CollisionHandler) {
        let render_data_holder = self.collision_task_wrapper.update_holder();

        let mut hm: FxHashMap<String, Vec<(DVec3, f32)>> = FxHashMap::default();
        let mut id_collisions: FxHashSet<u64> = FxHashSet::default();
        render_data_holder
            .run_mut_action(|(shape_info, _, key)| {

                if id_collisions.insert(shape_info.id) {
                    let mut alpha = *self.id_to_alpha.get(&shape_info.id).unwrap_or(&0.0f32);
                    let screen_pos = view_projection.screen_position(&shape_info.position);
                    let offset = shape_info.size * 0.67;
                    // no need to use f64 for collision detection
                    let bounds = Rectangle::from_corners(
                        point! { x: screen_pos.x as f32 - offset, y: screen_pos.y as f32 - offset},
                        point! { x: screen_pos.x as f32 + offset, y: screen_pos.y as f32 + offset},
                    );

                    let within_screen = collision_handler.within_screen(bounds);
                    let prev_alpha = alpha;
                    if within_screen {
                        if collision_handler.check_and_insert(bounds) {
                            alpha = clamp(alpha + Self::FADE_ANIM_SPEED, 0.0, 1.0);
                        } else {
                            alpha = clamp(alpha - Self::FADE_ANIM_SPEED, 0.0, 1.0);
                        }
                        self.id_to_alpha.insert(shape_info.id, alpha);
                    }

                    // don't process if it was transparent and nothing changed
                    let still_transparent = prev_alpha == 0.0 && prev_alpha == alpha;
                    if !still_transparent {
                        hm.entry(key.clone()).or_default().push((shape_info.position, alpha));
                    }
                }
            });

        self.id_to_alpha.retain(|id, _| id_collisions.contains(id));
        self.collision_task_wrapper.send_result(hm);
    }
}
