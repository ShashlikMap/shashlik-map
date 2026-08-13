use crate::global_context::GlobalContext;
use crate::mesh::InstanceBuffer;
use crate::mesh::mesh::Mesh;
use crate::mesh::mesh_instance_input::MeshInstanceInput;
use crate::mesh_buffers::MeshBuffers;
use crate::utils::ReceiverExt;
use glam::DVec3;
use renderer_common::render_modifier::SpatialData;
use tokio::sync::broadcast::Receiver;
use wgpu::util::{DeviceExt, DrawIndexedIndirectArgs};
use wgpu::{ComputePass, RenderPass};

pub struct PositionedMesh<T: MeshInstanceInput> {
    mesh: Mesh,
    instance_buffer: InstanceBuffer<T>,
    attrs: Vec<T>,
    cs_offset: DVec3,
    double_style: bool,
    spatial_rx: Receiver<SpatialData>,
    original_spatial_data: SpatialData,
    original_instance_positions_alpha: Vec<(DVec3, f32)>,
    mesh_buffers: MeshBuffers
}

impl Mesh {
    pub fn to_positioned<T: MeshInstanceInput>(
        self,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        double_style: bool,
        instance_positions_alpha: Option<Vec<(DVec3, f32)>>,
    ) -> PositionedMesh<T> {
        PositionedMesh::new(self, spatial_rx, double_style, instance_positions_alpha)
    }
}

impl<T: MeshInstanceInput> PositionedMesh<T> {
    pub fn new(
        mesh: Mesh,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        double_style: bool,
        instance_positions_alpha: Option<Vec<(DVec3, f32)>>,
    ) -> Self {
        Self {
            mesh,
            instance_buffer: InstanceBuffer::default(),
            attrs: vec![],
            cs_offset: DVec3::new(0.0, 0.0, 0.0),
            double_style,
            spatial_rx,
            original_spatial_data: SpatialData::new(),
            original_instance_positions_alpha: instance_positions_alpha
                .unwrap_or(vec![(DVec3::new(0.0, 0.0, 0.0), 1f32)]),
            mesh_buffers: MeshBuffers::default(),
        }
    }

    pub fn update(
        &mut self,
        global_context: &mut GlobalContext,
        indirect: bool,
    ) {
        let cs_offset_updated = global_context.view_projection.cs_offset != self.cs_offset;
        self.cs_offset = global_context.view_projection.cs_offset;
        let mut update_attrs = cs_offset_updated;

        if let Ok(spatial_data) = self.spatial_rx.no_lagged() {
            self.original_spatial_data = spatial_data;
            update_attrs = true;
        }

        if update_attrs {
            T::fill_attrs(
                &mut self.attrs,
                &self.cs_offset,
                &self.original_instance_positions_alpha,
                &self.original_spatial_data,
            );
            self.instance_buffer.update(
                "PositionedInstanceBuffer",
                global_context,
                &self.attrs,
            );

            if indirect {
                let instance_buffer_length = self.get_instance_buffer_length();
                let culled_buffer = global_context.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Culled Buffer"),
                    contents: bytemuck::cast_slice(&vec![0; instance_buffer_length]),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });
                
                let instance_count = if self.attrs.len() <= 2 {
                    self.attrs.len() * instance_buffer_length
                } else {
                    0
                };
                let index_count = if instance_count == 0 {
                    6
                } else {
                    self.mesh.index_buf.1
                };
                // instances_args_buffer has to be reset before computing
                let indirect_args_struct = DrawIndexedIndirectArgs {
                    index_count: index_count as u32,
                    instance_count: instance_count as u32,
                    first_index: 0,
                    base_vertex: 0,
                    first_instance: 0,
                };
                let indirect_args = global_context.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("indirect args"),
                    contents: indirect_args_struct.as_bytes(),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT |wgpu::BufferUsages::COPY_DST,
                });

                if let Some(instance_buffer) = self.instance_buffer.buffer.as_ref() {
                    self.mesh_buffers = MeshBuffers::builder()
                        .with_instance_buffer(Some(instance_buffer.clone()))
                        .with_culled_and_args_buffer(Some(culled_buffer), Some(indirect_args))
                }

            } else {
                self.mesh_buffers = MeshBuffers::builder()
                    .with_instance_buffer(self.instance_buffer.buffer.clone())
            }
        }
    }

    pub fn compute_instanced(
        &self,
        compute_pass: &mut ComputePass,
    ) {
        if self.mesh_buffers.instance_buffer().is_some() {
            // workgroups are batches by 64/128
            let instance_buffer_length = self.get_instance_buffer_length() as u32;
            let mut x = instance_buffer_length / 64;
            if instance_buffer_length % 64 != 0 {
                x += 1; // those are workgroups, not invocations.
            }
            compute_pass.dispatch_workgroups(x, 1, 1);
        }
    }

    fn get_instance_buffer_length(&self) -> usize {
        let factor = if self.double_style { 2 } else { 1 };
        self.instance_buffer.length * factor
    }

    pub fn get_mesh_buffers(&self) -> &MeshBuffers {
        &self.mesh_buffers
    }

    pub fn render_instanced(
        &mut self,
        render_pass: &mut RenderPass,
        disable_skip_mesh_feature: bool,
    ) {
        let instances_args_buffer = self.mesh_buffers.args_buffer();
        let instance_count = self.get_instance_buffer_length();
        self.mesh.render_instanced(
            render_pass,
            instance_count,
            disable_skip_mesh_feature,
            instances_args_buffer
        );
    }
}
