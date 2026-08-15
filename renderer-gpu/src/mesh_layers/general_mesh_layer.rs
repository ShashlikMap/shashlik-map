use crate::buffer_pool::BufferPool;
use crate::draw_commands::mesh2d_draw_command::Mesh2dCommandBatch;
use crate::global_context::GlobalContext;
use crate::mesh::mesh::Mesh;
use crate::mesh::positioned_mesh::PositionedMesh;
use crate::mesh_layers::render_data_holder::RenderDataHolder;
use crate::mesh_layers::{BaseMeshComputeLayerNew, BaseMeshLayer, BaseMeshLayerNew};
use crate::pipelines::RenderPipeline;
use renderer_common::render_modifier::SpatialData;
use std::mem;
use wgpu::{CommandEncoder, ComputePassDescriptor, RenderPass};
use crate::mesh::mesh_instance_input::MeshInstanceInput;

pub(crate) struct GeneralMeshLayer<I: MeshInstanceInput> {
    render_data_holder: RenderDataHolder<PositionedMesh<I>>,
    indirect: bool,
    pub disable_skip_mesh_feature: bool,
}

impl<I: MeshInstanceInput> GeneralMeshLayer<I> {
    pub fn new(indirect: bool) -> Self {
        GeneralMeshLayer {
            render_data_holder: RenderDataHolder::new(),
            indirect,
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
        let mesh = mesh.to_positioned(spatial_rx,
                                      double_style,
                                      None);
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
        
        let mesh = mesh.to_positioned(spatial_rx,
                                      batch.mesh_info.double_style,
                                      instance_positions);
        self.render_data_holder.set(key.to_string(), vec![mesh]);
    }
}

impl<I: MeshInstanceInput> BaseMeshLayer for GeneralMeshLayer<I> {

    fn update(&mut self, global_context: &mut GlobalContext) {
        self.render_data_holder.run_mut_action(|mesh| {
            mesh.update(global_context, self.indirect);
        });
    }

    fn clear_by_key(&mut self, key: &str) {
        self.render_data_holder.remove(key);
    }
}

impl<I: MeshInstanceInput> BaseMeshLayerNew<I> for GeneralMeshLayer<I> {
    fn render_new(&mut self, render_pass: &mut RenderPass, render_pipeline: &mut impl RenderPipeline<I>, global_context: &mut GlobalContext) {
        render_pipeline.setup_render(render_pass, global_context);
        self.render_data_holder.run_mut_action(|mesh| {
            render_pipeline.setup_mesh_buffers(render_pass, mesh.get_mesh_buffers());
            mesh.render_instanced(render_pass, self.disable_skip_mesh_feature);
        });
    }
}

impl<I: MeshInstanceInput> BaseMeshComputeLayerNew<I> for GeneralMeshLayer<I> {
    fn compute_new(&mut self, command_encoder: &mut CommandEncoder, render_pipeline: &mut impl RenderPipeline<I>, global_context: &mut GlobalContext) {
        let mut compute_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("General Mesh Layer Compute Pass"),
            timestamp_writes: None,
        });

        render_pipeline.setup_compute(&mut compute_pass, global_context);
        self.render_data_holder.run_mut_action(|mesh| {
            render_pipeline.compute_mesh(&mut compute_pass, mesh.get_mesh_buffers());
            mesh.compute_instanced(&mut compute_pass);
        });
    }
}