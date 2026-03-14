use crate::draw_commands::mesh2d_draw_command::Mesh2dCommandBatch;
use crate::global_context::GlobalContext;
use crate::mesh::mesh::Mesh;
use crate::mesh::positioned_mesh::PositionedMesh;
use crate::mesh_layers::render_data_holder::RenderDataHolder;
use crate::mesh_layers::BaseMeshLayer;
use crate::modifier::render_modifier::SpatialData;
use crate::pipelines::RenderPipeline;
use std::mem;
use wgpu::{CommandEncoder, ComputePassDescriptor, RenderPass};
use wgpu::TextureFormat::Rgba16Float;

pub(crate) struct GeneralMeshLayer<P: RenderPipeline> {
    render_pipeline: P,
    pipeline: Option<wgpu::RenderPipeline>,
    g_buffer_pipeline: Option<wgpu::RenderPipeline>,
    render_data_holder: RenderDataHolder<PositionedMesh<P::InstanceInputType>>,
    pub disable_skip_mesh_feature: bool,
}

impl<P: RenderPipeline> GeneralMeshLayer<P> {
    pub fn new(render_pipeline: P) -> Self {
        GeneralMeshLayer {
            render_pipeline,
            pipeline: None,
            g_buffer_pipeline: None,
            render_data_holder: RenderDataHolder::new(),
            disable_skip_mesh_feature: false,
        }
    }
    pub fn add(
        &mut self,
        key: String,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        double_style: bool,
        mesh: Mesh,
    ) {

        let mesh = P::create_positioned_mesh(
            spatial_rx,
            double_style,
            None,
            mesh,
        );
        self.render_data_holder.set(key, vec![mesh]);
    }

    pub fn submit_batch(
        &mut self,
        key: &str,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        global_context: &mut GlobalContext,
        batch: &mut Mesh2dCommandBatch,
    ) {
        let device = global_context.device();
        let mesh = Mesh::create_layered(
            &device,
            &batch.mesh,
            mem::take(&mut batch.layers_indices),
        );

        let instance_positions =
            mem::take(&mut batch.mesh_info.instance_positions).map(|pos_items| pos_items.into_iter().map(|item| {
                (item, 1f32)
            }).collect());

        let mesh = P::create_positioned_mesh(
            spatial_rx,
            batch.mesh_info.double_style,
            instance_positions,
            mesh,
        );
        self.render_data_holder.set(key.to_string(), vec![mesh]);
    }
}

impl<P: RenderPipeline> BaseMeshLayer for GeneralMeshLayer<P> {
    fn prepare(&mut self, global_context: &GlobalContext) {
        let descriptor = self.render_pipeline.prepare(global_context);
        let mut g_buffer_descriptor = descriptor.clone();

        self.pipeline = Some(descriptor.to_render_pipeline(global_context.device()));

        g_buffer_descriptor.label = Some("g_buffer_pipeline");
        let fragment = g_buffer_descriptor.fragment.as_mut().unwrap();
        fragment.targets = vec![Some(wgpu::ColorTargetState {
            format: Rgba16Float,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        }), Some(wgpu::ColorTargetState {
            format: Rgba16Float,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        fragment.entry_point = Some("fs_main_g_buf");
        g_buffer_descriptor.multisample.count = 1;
        self.g_buffer_pipeline = Some(g_buffer_descriptor.to_render_pipeline(global_context.device()));
    }

    fn update(&mut self, global_context: &mut GlobalContext) {
        self.render_data_holder.run_mut_action(|mesh| {
            mesh.update(global_context, self.render_pipeline.get_instances_layouts());
        });
    }

    fn compute(&mut self, encoder: &mut CommandEncoder, global_context: &mut GlobalContext) {
        if self.render_pipeline.is_indirect() {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Indirect General Mesh Layer Compute Pass"),
                timestamp_writes: None,
            });
            self.render_pipeline.compute(&mut compute_pass, global_context);
            self.render_data_holder.run_mut_action(|mesh| {
                if let (Some(instance_bind_group),
                    Some(instance_args_bind_group)) = (mesh.instances_bind_group.as_ref(),
                                                       mesh.instances_args_bind_group.as_ref()) {
                    self.render_pipeline.set_instance_bind_group_compute(&mut compute_pass,
                                                                         instance_bind_group,
                                                                         instance_args_bind_group);
                    mesh.compute_instanced(&mut compute_pass, global_context);
                }
            });
        }
    }

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        if let Some(render_pipeline) = self.pipeline.as_ref() {
            if global_context.is_g_buffer_render {
                render_pass.set_pipeline(self.g_buffer_pipeline.as_ref().unwrap());
            } else {
                render_pass.set_pipeline(render_pipeline);
            }

            self.render_pipeline.render(render_pass, global_context);

            self.render_data_holder.run_mut_action(|mesh| {
                if self.render_pipeline.is_indirect()
                    && let Some(instance_bind_group) = mesh.instances_bind_group.as_ref() {
                    self.render_pipeline.set_instance_bind_group_render(render_pass, instance_bind_group);
                }
                mesh.render_instanced(render_pass, self.disable_skip_mesh_feature);
            });
        }
    }

    fn clear_by_key(&mut self, key: &str) {
        self.render_data_holder.remove(key);
    }
}
