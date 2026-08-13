use crate::buffer_pool::BufferPool;
use crate::collider::{ColliderTask, CollisionTaskController, CollisionTaskWrapper};
use crate::draw_commands::mesh2d_draw_command::Mesh2dDrawCommand;
use crate::global_context::GlobalContext;
use crate::mesh::InstanceBuffer;
use crate::mesh::mesh::Mesh;
use crate::mesh::mesh_instance_input::MeshInstanceInput;
use crate::mesh_buffers::MeshBuffers;
use crate::mesh_layers::BaseMeshLayer;
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
use wgpu::{CommandEncoder, RenderPass};

// TODO ScreenMeshLayer and GeneralMeshLayer could be combined somehow.
pub(crate) struct ScreenShapeLayer<P: RenderPipeline> {
    render_pipeline: P,
    pipeline: Option<wgpu::RenderPipeline>,
    meshes: HashMap<String, (Mesh, InstanceBuffer<P::InstanceInputType>, MeshBuffers)>,
    collision_task_controller: CollisionTaskController<
        (ShapeInfo, f32, String),
        HashMap<String, Vec<(DVec3, f32)>>,
    >,
}

struct ShapeInfo {
    pub position: DVec3,
    pub size: f32,
}

impl<P: RenderPipeline> ScreenShapeLayer<P> {
    pub fn new(render_pipeline: P, global_context: &mut GlobalContext) -> Self {
        let (task_wrapper, collision_task_controller) = CollisionTaskWrapper::new();
        let task = ScreenMeshCollisionHandler::new(task_wrapper);
        global_context.collider.register_task(Box::new(task));
        ScreenShapeLayer {
            render_pipeline,
            pipeline: None,
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
                    InstanceBuffer::default(),
                    MeshBuffers::default()
                )
            });

            let instance_positions =
                mem::take(&mut batch.mesh_info.instance_positions).unwrap_or_default();

            let size = batch.mesh_info.size.unwrap_or_default();


            let batch_data: Vec<_> = instance_positions.into_iter().map(|item| {
                let shape_info = ShapeInfo {
                    position: item + spatial_data.transform,
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

impl<P: RenderPipeline> BaseMeshLayer for ScreenShapeLayer<P> {
    fn prepare(&mut self, global_context: &GlobalContext) {
        let descriptor = self.render_pipeline.prepare(global_context);
        self.pipeline = Some(descriptor.to_render_pipeline(global_context.device()));
    }

    fn update(&mut self, global_context: &mut GlobalContext) {
        let Ok(hm) = self.collision_task_controller.receiver.try_recv() else {
            return;
        };
        let cs_offset = global_context.view_projection.cs_offset;
        self.meshes
            .iter_mut()
            .for_each(|(key, (_, instance_buffer, mesh_buffers))| {
                let mut attrs = Vec::new();
                if let Some(pos_alpha) = hm.get(key) {
                    P::InstanceInputType::fill_attrs(
                        &mut attrs,
                        &cs_offset,
                        pos_alpha,
                        &SpatialData::new(),
                    );
                }

                instance_buffer.update(
                    "ScreenInstanceBuffer",
                    global_context,
                    &attrs,
                );
                *mesh_buffers = MeshBuffers::builder().with_instance_buffer(instance_buffer.buffer.clone());
            });
    }


    fn compute(&mut self, _encoder: &mut CommandEncoder,_global_context: &mut GlobalContext) {}

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        if let Some(render_pipeline) = self.pipeline.as_ref() {
            render_pass.set_pipeline(render_pipeline);

            self.render_pipeline.setup_render(render_pass, global_context);

            self.meshes.iter().for_each(|(_, (mesh, instance_buf, mesh_buffers))| {
                self.render_pipeline.render_mesh(render_pass, mesh_buffers, global_context);
                let instance_count = instance_buf.length;
                mesh.render_instanced(render_pass, instance_count, false, None);
            });
        }
    }

    fn clear_by_key(&mut self, key: &str) {
        self.collision_task_controller.clear_by_key(key);
    }
}

struct ScreenMeshCollisionHandler {
    collision_task_wrapper: CollisionTaskWrapper<
        (ShapeInfo, f32, String),
        HashMap<String, Vec<(DVec3, f32)>>,
    >,
}

impl ScreenMeshCollisionHandler {
    const FADE_ANIM_SPEED: f32 = 0.05;
    pub fn new(
        collision_task_wrapper: CollisionTaskWrapper<
            (ShapeInfo, f32, String),
            HashMap<String, Vec<(DVec3, f32)>>,
        >,
    ) -> Self {
        ScreenMeshCollisionHandler {
            collision_task_wrapper,
        }
    }
}

impl ColliderTask for ScreenMeshCollisionHandler {
    fn run(&mut self, view_projection: &ViewProjection, collision_handler: &mut CollisionHandler) {
        let render_data_holder = self.collision_task_wrapper.update_holder();

        let mut hm: HashMap<String, Vec<(DVec3, f32)>> = HashMap::new();
        render_data_holder
            .run_mut_action(|(shape_info, alpha, key)| {
                let screen_pos = view_projection.screen_position(&shape_info.position);
                let offset = shape_info.size * 0.5;
                // no need to use f64 for collision detection
                let bounds = Rectangle::from_corners(
                    point! { x: screen_pos.x as f32 - offset, y: screen_pos.y as f32 - offset},
                    point! { x: screen_pos.x as f32 + offset, y: screen_pos.y as f32 + offset},
                );

                let within_screen = collision_handler.within_screen(bounds);
                if within_screen {
                    if collision_handler.check_and_insert(bounds) {
                        *alpha = clamp(*alpha + Self::FADE_ANIM_SPEED, 0.0, 1.0);
                    } else {
                        *alpha = clamp(*alpha - Self::FADE_ANIM_SPEED, 0.0, 1.0);
                    }
                }

                hm.entry(key.clone()).or_default().push((shape_info.position, *alpha));
            });

        self.collision_task_wrapper.send_result(hm);
    }
}
