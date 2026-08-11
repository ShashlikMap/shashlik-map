use crate::draw_commands::mesh2d_draw_command::Mesh2dCommandBatch;
use crate::global_context::{GlobalContext, GlobalRenderStep};
use crate::mesh::mesh::Mesh;
use crate::mesh::positioned_mesh::PositionedMesh;
use crate::mesh_layers::render_data_holder::RenderDataHolder;
use crate::mesh_layers::BaseMeshLayer;
use renderer_common::render_modifier::SpatialData;
use crate::pipelines::RenderPipeline;
use std::mem;
use wgpu::TextureFormat::Rgba16Float;
use wgpu::{CommandEncoder, ComputePassDescriptor, Face, RenderPass, TextureFormat};
use crate::buffer_pool::BufferPool;

pub(crate) struct GeneralMeshLayer<P: RenderPipeline> {
    render_pipeline: P,
    pipeline: Option<wgpu::RenderPipeline>,
    g_buffer_pipeline: Option<wgpu::RenderPipeline>,
    shadow_pipeline: Option<wgpu::RenderPipeline>,
    render_data_holder: RenderDataHolder<PositionedMesh<P::InstanceInputType>>,
    pub disable_skip_mesh_feature: bool,
    write_to_stencil: bool,
}

impl<P: RenderPipeline> GeneralMeshLayer<P> {
    pub fn new(render_pipeline: P, write_to_stencil: bool) -> Self {
        GeneralMeshLayer {
            render_pipeline,
            pipeline: None,
            g_buffer_pipeline: None,
            shadow_pipeline: None,
            render_data_holder: RenderDataHolder::new(),
            disable_skip_mesh_feature: false,
            write_to_stencil
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
        buffer_pool: &mut BufferPool,
        batch: &mut Mesh2dCommandBatch,
    ) {
        let mesh = Mesh::create_layered(
            Some(key),
            global_context,
            buffer_pool,
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
        let mut descriptor = self.render_pipeline.prepare(global_context);
        let mut g_buffer_descriptor = descriptor.clone();
        let mut shadow_descriptor = descriptor.clone();
        if self.write_to_stencil {
            descriptor.depth_stencil.as_mut().unwrap().stencil = wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Always,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Replace,
                },
                back: wgpu::StencilFaceState::default(),
                read_mask: 0xFF,
                write_mask: 0xFF,
            };
        }

        self.pipeline = Some(descriptor.to_render_pipeline(global_context.device()));

        if self.render_pipeline.support_g_buf() {
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
            // render pass for g buffer uses Depth24Plus but original descriptor Depth24PlusStencil8
            g_buffer_descriptor.depth_stencil.as_mut().unwrap().format = TextureFormat::Depth24Plus;
            self.g_buffer_pipeline = Some(g_buffer_descriptor.to_render_pipeline(global_context.device()));
        }

        shadow_descriptor.label = Some("shadow_pipeline");
        shadow_descriptor.fragment = None;
        shadow_descriptor.primitive.cull_mode = Some(Face::Front);
        shadow_descriptor.multisample.count = 1;
        shadow_descriptor.depth_stencil.as_mut().unwrap().format = wgpu::TextureFormat::Depth32Float;
        self.shadow_pipeline = Some(shadow_descriptor.to_render_pipeline(global_context.device()));
    }

    fn update(&mut self, global_context: &mut GlobalContext) {
        self.render_data_holder.run_mut_action(|mesh| {
            mesh.update(global_context, self.render_pipeline.is_indirect());
        });
    }

    fn compute(&mut self, encoder: &mut CommandEncoder, global_context: &mut GlobalContext) {
        if self.render_pipeline.is_indirect() {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Indirect General Mesh Layer Compute Pass"),
                timestamp_writes: None,
            });

            self.render_data_holder.run_mut_action(|mesh| {
                self.render_pipeline.compute_mesh(&mut compute_pass, mesh.get_mesh_buffers(), global_context);
                mesh.compute_instanced(&mut compute_pass);
            });
        }
    }

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        if let Some(render_pipeline) = self.pipeline.as_ref() {
            if global_context.check_render_step(GlobalRenderStep::GBufferStep) && let Some(g_buffer_pipeline) = self.g_buffer_pipeline.as_ref() {
                render_pass.set_pipeline(g_buffer_pipeline);
            } else if global_context.check_render_step(GlobalRenderStep::ShadowStep) {
                render_pass.set_pipeline(self.shadow_pipeline.as_ref().unwrap());
            } else {
                render_pass.set_pipeline(render_pipeline);
                if self.write_to_stencil {
                    render_pass.set_stencil_reference(1);
                }
            }

            self.render_pipeline.setup_render(render_pass, global_context);

            self.render_data_holder.run_mut_action(|mesh| {
                self.render_pipeline.render_mesh(render_pass, mesh.get_mesh_buffers(), global_context);
                mesh.render_instanced(render_pass, self.disable_skip_mesh_feature);
            });
        }
    }

    fn clear_by_key(&mut self, key: &str) {
        self.render_data_holder.remove(key);
    }
}
