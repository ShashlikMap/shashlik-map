use crate::buffer_pool::BufferPool;
use crate::draw_commands::mesh2d_draw_command::Mesh2dCommandBatch;
use crate::global_context::GlobalContext;
use crate::mesh::mesh::Mesh;
use crate::mesh::positioned_mesh::PositionedMesh;
use crate::mesh_layers::render_data_holder::RenderDataHolder;
use crate::mesh_layers::BaseMeshLayer;
use crate::mesh_layers::ComputableLayer;
use crate::mesh_layers::LayerAttrMapper;
use crate::pipelines::RenderPipeline;
use renderer_common::render_modifier::SpatialData;
use std::mem;
use wgpu::{CommandEncoder, ComputePassDescriptor, RenderPass};
use crate::mesh::mesh_instance_input::{MeshInstanceInput};
use crate::mesh::virtual_ground_circle_mesh::VirtualGroundCircleMesh;
use crate::mesh::virtual_ground_mesh::VirtualGroundMesh;

pub(crate) struct GeneralMeshLayer<I: MeshInstanceInput> {
    render_data_holder: RenderDataHolder<PositionedMesh<I>>,
    indirect: bool,
    pub disable_skip_mesh_feature: bool,
    attr_map: LayerAttrMapper<I>,
    virtual_ground: Option<VirtualGroundMesh>,
    virtual_ground_circle: Option<VirtualGroundCircleMesh>,
}

impl<I: MeshInstanceInput> GeneralMeshLayer<I> {
    pub fn new(indirect: bool, attr_map: LayerAttrMapper<I>) -> Self {
        GeneralMeshLayer {
            render_data_holder: RenderDataHolder::new(),
            indirect,
            disable_skip_mesh_feature: false,
            attr_map,
            virtual_ground: None,
            virtual_ground_circle: None,
        }
    }
    pub fn add(
        &mut self,
        key: String,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        double_style: bool,
        mesh: Mesh,
    ) {
        let mesh = mesh.to_positioned(self.attr_map, spatial_rx,
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
                (item.1, 1f32)
            }).collect());

        let mesh = mesh.to_positioned(self.attr_map,
                                      spatial_rx,
                                      batch.mesh_info.double_style,
                                      instance_positions);
        self.render_data_holder.set(key.to_string(), vec![mesh]);
    }

    pub fn set_virtual_ground(&mut self, global_context: &GlobalContext, buffer_pool: &mut BufferPool) {
        self.virtual_ground_circle = None;
        self.virtual_ground = None;
        if global_context.view_projection.round_screen_sq_radius().is_some() {
            self.virtual_ground_circle = Some(VirtualGroundCircleMesh::new(global_context, buffer_pool));
        } else {
            self.virtual_ground = Some(VirtualGroundMesh::new(global_context, buffer_pool));
        }
    }

    pub fn render(&mut self, render_pass: &mut RenderPass,
                  render_pipeline: &mut impl RenderPipeline<I>,
                  global_context: &mut GlobalContext) {
        self.render_with_virtual_ground(render_pass, render_pipeline, global_context, false);
    }

    pub fn render_with_virtual_ground(&mut self, render_pass: &mut RenderPass,
              render_pipeline: &mut impl RenderPipeline<I>,
              global_context: &mut GlobalContext,
              virtual_ground_enabled: bool) {
        render_pipeline.setup_render(render_pass, global_context);
        if !render_pipeline.is_mesh_rendering_enabled() {
            return;
        }
        self.render_data_holder.run_mut_action(|mesh| {
            render_pipeline.setup_mesh_buffers(render_pass, mesh.get_mesh_buffers());
            mesh.render_instanced(render_pass, global_context, self.disable_skip_mesh_feature);
        });

        // we render virtual_ground after other meshes to utilize depth buffer to cull geometry
        if virtual_ground_enabled {
            if let Some(virtual_ground) = self.virtual_ground.as_mut() {
                virtual_ground.render(render_pass);
            } else if let Some(virtual_ground) = self.virtual_ground_circle.as_mut() {
                virtual_ground.render(render_pass);
            }
        }
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

impl<I: MeshInstanceInput> ComputableLayer<I> for GeneralMeshLayer<I> {
    fn compute(&mut self, command_encoder: &mut CommandEncoder, render_pipeline: &mut impl RenderPipeline<I>, global_context: &mut GlobalContext) {
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