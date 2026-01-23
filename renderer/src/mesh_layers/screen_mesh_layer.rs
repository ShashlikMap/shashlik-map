use crate::draw_commands::mesh2d_draw_command::Mesh2dDrawCommand;
use crate::global_context::GlobalContext;
use crate::mesh::InstanceBuffer;
use crate::mesh::mesh::Mesh;
use crate::mesh::mesh_instance_input::MeshInstanceInput;
use crate::mesh_layers::BaseMeshLayer;
use crate::mesh_layers::render_data_holder::RenderDataHolder;
use crate::modifier::render_modifier::SpatialData;
use crate::pipelines::RenderPipeline;
use cgmath::Vector3;
use cgmath::num_traits::clamp;
use geo_types::point;
use rstar::primitives::Rectangle;
use std::collections::HashMap;
use std::mem;
use wgpu::{Device, RenderPass};

// TODO ScreenMeshLayer and GeneralMeshLayer could be combined somehow.
pub(crate) struct ScreenMeshLayer<P: RenderPipeline> {
    render_pipeline: P,
    pipeline: Option<wgpu::RenderPipeline>,
    meshes: HashMap<String, (Mesh, InstanceBuffer<P::InstanceInputType>)>,
    instance_data: RenderDataHolder<(Vector3<f64>, f32, String)>,
}

impl<P: RenderPipeline> ScreenMeshLayer<P> {
    pub fn new(render_pipeline: P) -> Self {
        ScreenMeshLayer {
            render_pipeline,
            pipeline: None,
            meshes: HashMap::new(),
            instance_data: RenderDataHolder::new(),
        }
    }

    pub fn submit(
        &mut self,
        key: &str,
        spatial_data: SpatialData,
        device: &Device,
        command: &mut Mesh2dDrawCommand,
    ) {
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
            self.instance_data.add(
                key.to_string(),
                (item + spatial_data.transform, 0.0, instance_key.clone()),
            );
        });
    }
}

impl<P: RenderPipeline> BaseMeshLayer for ScreenMeshLayer<P> {
    fn prepare(&mut self, global_context: &GlobalContext) {
        let descriptor = self.render_pipeline.prepare(global_context);
        self.pipeline = Some(descriptor.to_render_pipeline(global_context.device()));
    }

    fn update(&mut self, global_context: &mut GlobalContext) {
        // TODO Need to check which are not collidable(but no use case yet)

        let mut hm: HashMap<String, Vec<(Vector3<f64>, f32)>> = HashMap::new();
        self.instance_data.run_mut_action(|(pos, alpha, key)| {
            let screen_pos = global_context.view_projection.screen_position(&pos);
            // TODO Bounds for svg?
            // no need to use f64 for collision detection
            let bounds = Rectangle::from_corners(
                point! { x: screen_pos.x as f32 - 20.0, y: screen_pos.y as f32 - 20.0},
                point! { x: screen_pos.x as f32+ 20.0, y: screen_pos.y as f32 + 20.0},
            );

            let within_screen = global_context.collision_handler.within_screen(bounds);
            if within_screen {
                if global_context.collision_handler.insert(bounds) {
                    *alpha = clamp(*alpha + 0.05, 0.0, 1.0);
                } else {
                    *alpha = clamp(*alpha - 0.05, 0.0, 1.0);
                }
            }

            hm.entry(key.clone()).or_default().push((*pos, *alpha));
        });

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
        self.instance_data.remove(key);
    }
}
