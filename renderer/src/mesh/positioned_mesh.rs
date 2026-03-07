use crate::global_context::GlobalContext;
use crate::mesh::InstanceBuffer;
use crate::mesh::mesh::Mesh;
use crate::mesh::mesh_instance_input::MeshInstanceInput;
use crate::modifier::render_modifier::SpatialData;
use crate::utils::ReceiverExt;
use cgmath::Vector3;
use tokio::sync::broadcast::Receiver;
use wgpu::{BindGroup, BindGroupLayout, Buffer, ComputePass, RenderPass};
use wgpu::util::{DeviceExt, DrawIndexedIndirectArgs};

pub struct PositionedMesh<T: MeshInstanceInput> {
    mesh: Mesh,
    instance_buffer: InstanceBuffer<T>,
    attrs: Vec<T>,
    cs_offset: Vector3<f64>,
    double_style: bool,
    spatial_rx: Receiver<SpatialData>,
    original_spatial_data: SpatialData,
    original_instance_positions_alpha: Vec<(Vector3<f64>, f32)>,
    pub instances_args_buffer: Option<Buffer>,
    pub instances_bind_group: Option<BindGroup>,
    pub instances_args_bind_group: Option<BindGroup>,
}

impl Mesh {
    pub fn to_positioned<T: MeshInstanceInput>(
        self,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        double_style: bool,
        instance_positions_alpha: Option<Vec<(Vector3<f64>, f32)>>,
    ) -> PositionedMesh<T> {
        PositionedMesh::new(self, spatial_rx, double_style, instance_positions_alpha)
    }
}

impl<T: MeshInstanceInput> PositionedMesh<T> {
    pub fn new(
        mesh: Mesh,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        double_style: bool,
        instance_positions_alpha: Option<Vec<(Vector3<f64>, f32)>>,
    ) -> Self {
        Self {
            mesh,
            instance_buffer: InstanceBuffer::default(),
            attrs: vec![],
            cs_offset: Vector3::new(0.0, 0.0, 0.0),
            double_style,
            spatial_rx,
            original_spatial_data: SpatialData::new(),
            original_instance_positions_alpha: instance_positions_alpha
                .unwrap_or(vec![(Vector3::new(0.0, 0.0, 0.0), 1f32)]),
            instances_args_buffer: None,
            instances_bind_group: None,
            instances_args_bind_group: None
        }
    }

    pub fn update(
        &mut self,
        global_context: &mut GlobalContext,
        instances_bind_group_layout: Option<(&BindGroupLayout, &BindGroupLayout)>,
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
                self.double_style,
            );
            self.instance_buffer.update(
                "PositionedInstanceBuffer",
                global_context.device(),
                global_context.queue(),
                &self.attrs,
            );

            if let Some(instances_bind_group_layout) = instances_bind_group_layout {

                let culled_buffer = global_context.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Culled Buffer"),
                    contents: bytemuck::cast_slice(&vec![0;  self.instance_buffer.length]),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

                self.instances_bind_group = Some(
                    global_context
                        .device()
                        .create_bind_group(&wgpu::BindGroupDescriptor {
                            layout: instances_bind_group_layout.0,
                            entries: &[wgpu::BindGroupEntry {
                                binding: 0,
                                resource: self
                                    .instance_buffer
                                    .buffer
                                    .as_ref()
                                    .expect("Buffer should exist")
                                    .as_entire_binding(),
                            },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: culled_buffer.as_entire_binding(),
                                }],
                            label: Some("instances_bind_group"),
                        }),
                );

                let indirect_args_struct = DrawIndexedIndirectArgs {
                    index_count: 6,
                    instance_count: 0,
                    first_index: 0,
                    base_vertex: 0,
                    first_instance: 0,
                };
                let indirect_args = global_context.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("indirect args"),
                    contents: indirect_args_struct.as_bytes(),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT |wgpu::BufferUsages::COPY_DST,
                });

                self.instances_args_bind_group = Some(
                    global_context
                        .device()
                        .create_bind_group(&wgpu::BindGroupDescriptor {
                            layout: instances_bind_group_layout.1,
                            entries: &[wgpu::BindGroupEntry {
                                binding: 0,
                                resource: indirect_args
                                    .as_entire_binding(),
                            }],
                            label: Some("instances_bind_args_group"),
                        }),
                );
                self.instances_args_buffer = Some(indirect_args);
            } else {
                self.instances_bind_group = None;
                self.instances_args_bind_group = None;
            }
        }
    }

    pub fn compute_instanced(
        &mut self,
        compute_pass: &mut ComputePass,
        global_context: &mut GlobalContext,
    ) {
        if let Some(instances_args_buffer) = self.instances_args_buffer.as_ref() {
            // instances_args_buffer has to be reset before computing
            let indirect_args_struct = DrawIndexedIndirectArgs {
                index_count: 6,
                instance_count: 0,
                first_index: 0,
                base_vertex: 0,
                first_instance: 0,
            };
            global_context.queue().write_buffer(instances_args_buffer, 0, indirect_args_struct.as_bytes());
            compute_pass.dispatch_workgroups(self.instance_buffer.length as u32 / 64, 1, 1);
        }
    }

    pub fn render_instanced(
        &mut self,
        render_pass: &mut RenderPass,
        disable_skip_mesh_feature: bool,
    ) {
        let instances_vertex_slot = if self.instances_bind_group.is_some() {
            None
        } else {
            Some(1)
        };
        self.mesh.render_instanced(
            instances_vertex_slot,
            render_pass,
            &self.instance_buffer,
            disable_skip_mesh_feature,
            self.instances_args_buffer.as_ref()
        );
    }
}
