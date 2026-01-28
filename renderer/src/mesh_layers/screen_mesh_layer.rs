use crate::collider::ColliderTask;
use crate::collision_handler::CollisionHandler;
use crate::draw_commands::mesh2d_draw_command::Mesh2dDrawCommand;
use crate::global_context::GlobalContext;
use crate::mesh::InstanceBuffer;
use crate::mesh::mesh::Mesh;
use crate::mesh::mesh_instance_input::MeshInstanceInput;
use crate::mesh_layers::BaseMeshLayer;
use crate::mesh_layers::render_data_holder::RenderDataHolder;
use crate::modifier::render_modifier::SpatialData;
use crate::pipelines::RenderPipeline;
use crate::view_projection::ViewProjection;
use cgmath::Vector3;
use cgmath::num_traits::clamp;
use geo_types::point;
use rstar::primitives::Rectangle;
use std::collections::HashMap;
use std::mem;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use wgpu::RenderPass;

// TODO ScreenMeshLayer and GeneralMeshLayer could be combined somehow.
pub(crate) struct ScreenMeshLayer<P: RenderPipeline> {
    render_pipeline: P,
    pipeline: Option<wgpu::RenderPipeline>,
    meshes: HashMap<String, (Mesh, InstanceBuffer<P::InstanceInputType>)>,
    data_tx: Sender<
        Box<dyn FnOnce(&mut RenderDataHolder<(Vector3<f64>, f32, String)>) + Send + 'static>,
    >,
    result_rx: Receiver<HashMap<String, Vec<(Vector3<f64>, f32)>>>,
}

impl<P: RenderPipeline> ScreenMeshLayer<P> {
    pub fn new(render_pipeline: P, global_context: &mut GlobalContext) -> Self {
        let (data_tx, data_rx) = channel();
        let (result_tx, result_rx) = channel();

        let task = ScreenMeshCollisionHandler::new(data_rx, result_tx);
        global_context.collider.register_task(Box::new(task));
        ScreenMeshLayer {
            render_pipeline,
            pipeline: None,
            meshes: HashMap::new(),
            data_tx,
            result_rx,
        }
    }

    pub fn submit(
        &mut self,
        key: &str,
        spatial_data: SpatialData,
        global_context: &mut GlobalContext,
        command: &mut Mesh2dDrawCommand,
    ) {
        let device = global_context.device();
        let instance_key = command.mesh_info.instance_key.to_string();

        self.meshes.entry(instance_key.clone()).or_insert({
            (
                Mesh::create_layered(
                    &device,
                    &command.mesh,
                    mem::take(&mut command.layers_indices),
                ),
                InstanceBuffer::default(),
            )
        });

        let instance_positions =
            mem::take(&mut command.mesh_info.instance_positions).unwrap_or_default();

        instance_positions.into_iter().for_each(|item| {
            let key = key.to_string();
            let instance_key = instance_key.clone();
            self.data_tx
                .send(Box::new(move |holder| {
                    holder.add(
                        key.clone(),
                        (item + spatial_data.transform, 0.0f32, instance_key.clone()),
                    )
                }))
                .unwrap();
        });
    }
}

impl<P: RenderPipeline> BaseMeshLayer for ScreenMeshLayer<P> {
    fn prepare(&mut self, global_context: &GlobalContext) {
        let descriptor = self.render_pipeline.prepare(global_context);
        self.pipeline = Some(descriptor.to_render_pipeline(global_context.device()));
    }

    fn update(&mut self, global_context: &mut GlobalContext) {
        let Ok(hm) = self.result_rx.try_recv() else {
            return;
        };
        let cs_offset = global_context.view_projection.cs_offset;
        self.meshes
            .iter_mut()
            .for_each(|(key, (_, instance_buffer))| {
                let mut attrs = Vec::new();
                if let Some(pos_alpha) = hm.get(key) {
                    P::InstanceInputType::fill_attrs(
                        &mut attrs,
                        &cs_offset,
                        pos_alpha,
                        &SpatialData::new(),
                        false,
                    );
                }

                instance_buffer.update(
                    "ScreenInstanceBuffer",
                    global_context.device(),
                    global_context.queue(),
                    &attrs,
                )
            });
    }

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        if let Some(render_pipeline) = self.pipeline.as_ref() {
            render_pass.set_pipeline(render_pipeline);

            self.render_pipeline.render(render_pass, global_context);

            self.meshes.iter().for_each(|(_, (mesh, instance_buf))| {
                mesh.render_instanced(1, render_pass, instance_buf);
            });
        }
    }

    fn clear_by_key(&mut self, key: &str) {
        let key = key.to_string();
        self.data_tx
            .send(Box::new(move |holder| {
                holder.remove(key.as_str())
            }))
            .unwrap();
    }
}

struct ScreenMeshCollisionHandler {
    render_data_holder: RenderDataHolder<(Vector3<f64>, f32, String)>,
    data_rx: Arc<
        Mutex<
            Receiver<
                Box<
                    dyn FnOnce(&mut RenderDataHolder<(Vector3<f64>, f32, String)>) + Send + 'static,
                >,
            >,
        >,
    >,
    result_tx: Sender<HashMap<String, Vec<(Vector3<f64>, f32)>>>,
}

impl ScreenMeshCollisionHandler {
    const FADE_ANIM_SPEED: f32 = 0.05;
    pub fn new(
        data_receiver: Receiver<
            Box<dyn FnOnce(&mut RenderDataHolder<(Vector3<f64>, f32, String)>) + Send + 'static>,
        >,
        result_tx: Sender<HashMap<String, Vec<(Vector3<f64>, f32)>>>,
    ) -> Self {
        ScreenMeshCollisionHandler {
            render_data_holder: RenderDataHolder::new(),
            data_rx: Arc::new(Mutex::new(data_receiver)),
            result_tx,
        }
    }
}

impl ColliderTask for ScreenMeshCollisionHandler {
    fn run(&mut self, view_projection: &ViewProjection, collision_handler: &mut CollisionHandler) {
        while let Ok(data) = self.data_rx.lock().unwrap().try_recv() {
            data(&mut self.render_data_holder);
        }

        let mut hm: HashMap<String, Vec<(Vector3<f64>, f32)>> = HashMap::new();
        self.render_data_holder.run_mut_action(|(pos, alpha, key)| {
            let screen_pos = view_projection.screen_position(&pos);
            // TODO Bounds for svg?
            // no need to use f64 for collision detection
            let bounds = Rectangle::from_corners(
                point! { x: screen_pos.x as f32 - 20.0, y: screen_pos.y as f32 - 20.0},
                point! { x: screen_pos.x as f32 + 20.0, y: screen_pos.y as f32 + 20.0},
            );

            let within_screen = collision_handler.within_screen(bounds);
            if within_screen {
                if collision_handler.insert(bounds) {
                    *alpha = clamp(*alpha + Self::FADE_ANIM_SPEED, 0.0, 1.0);
                } else {
                    *alpha = clamp(*alpha - Self::FADE_ANIM_SPEED, 0.0, 1.0);
                }
            }

            hm.entry(key.clone()).or_default().push((*pos, *alpha));
        });

        self.result_tx.send(hm).unwrap();
    }
}
