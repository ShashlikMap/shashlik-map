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
use std::collections::hash_map::Entry;
use wgpu::RenderPass;

pub struct ScreenMeshLayer<P: RenderPipeline> {
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
        key: String,
        instance_key: &str,
        instance_positions: Vec<Vector3<f64>>,
        spatial_data: SpatialData,
        mesh: Mesh,
    ) {
        self.meshes
            .entry(instance_key.to_string())
            .or_insert((mesh, InstanceBuffer::default()));

        instance_positions.into_iter().for_each(|item| {
            self.instance_data.add(
                key.clone(),
                (item + spatial_data.transform, 0.0, instance_key.to_string()),
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
            let screen_pos = global_context.view_projection.screen_position(pos.clone());
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

            match hm.entry(key.clone()) {
                Entry::Occupied(mut entry) => entry.get_mut().push((*pos, *alpha)),
                Entry::Vacant(entry) => {
                    entry.insert(vec![(*pos, *alpha)]);
                }
            };
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
                if let Some(buf) = instance_buf.buffer.as_ref() {
                    render_pass.set_vertex_buffer(1, buf.slice(..));
                    let range = 0u32..instance_buf.length as u32;
                    mesh.render(render_pass, &range);
                }
            });
        }
    }

    fn clear_by_key(&mut self, key: &str) {
        self.instance_data.remove(key);
    }
}
