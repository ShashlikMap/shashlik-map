use crate::buffer_pool::BufferPool;
use crate::draw_commands::mesh2d_draw_command::Mesh2dCommandBatch;
use crate::global_context::GlobalContext;
use crate::mesh::mesh::Mesh;
use crate::mesh::positioned_mesh::PositionedMesh;
use crate::mesh_layers::render_data_holder::RenderDataHolder;
use crate::mesh_layers::{BaseMeshLayer, BaseMeshLayerNew};
use crate::pipelines::RenderPipeline;
use renderer_common::render_modifier::SpatialData;
use std::mem;
use wgpu::{CommandEncoder, ComputePassDescriptor, RenderPass};

pub(crate) struct GeneralMeshLayer<P: RenderPipeline> {
    render_pipeline: P,
    render_data_holder: RenderDataHolder<PositionedMesh<P::InstanceInputType>>,
    pub disable_skip_mesh_feature: bool,
}

impl<P: RenderPipeline> GeneralMeshLayer<P> {
    pub fn new(render_pipeline: P) -> Self {
        GeneralMeshLayer {
            render_pipeline,
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

            self.render_pipeline.setup_compute(&mut compute_pass, global_context);
            self.render_data_holder.run_mut_action(|mesh| {
                self.render_pipeline.compute_mesh(&mut compute_pass, mesh.get_mesh_buffers());
                mesh.compute_instanced(&mut compute_pass);
            });
        }
    }
    
    fn clear_by_key(&mut self, key: &str) {
        self.render_data_holder.remove(key);
    }
}

impl<P: RenderPipeline> BaseMeshLayerNew for GeneralMeshLayer<P> {
    fn render_new(&mut self, render_pass: &mut RenderPass, render_pipeline: &mut impl RenderPipeline, global_context: &mut GlobalContext) {
        render_pipeline.setup_render(render_pass, global_context);
        self.render_data_holder.run_mut_action(|mesh| {
            render_pipeline.setup_mesh_buffers(render_pass, mesh.get_mesh_buffers());
            mesh.render_instanced(render_pass, self.disable_skip_mesh_feature);
        });
    }
}