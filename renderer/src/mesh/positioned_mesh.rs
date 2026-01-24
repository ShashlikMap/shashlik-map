use crate::global_context::GlobalContext;
use crate::mesh::mesh::Mesh;
use crate::mesh::mesh_instance_input::MeshInstanceInput;
use crate::mesh::InstanceBuffer;
use crate::modifier::render_modifier::SpatialData;
use crate::utils::ReceiverExt;
use cgmath::Vector3;
use tokio::sync::broadcast::Receiver;
use wgpu::RenderPass;

pub struct PositionedMesh<T: MeshInstanceInput> {
    mesh: Mesh,
    instance_buffer: InstanceBuffer<T>,
    attrs: Vec<T>,
    cs_offset: Vector3<f64>,
    double_style: bool,
    spatial_rx: Receiver<SpatialData>,
    original_spatial_data: SpatialData,
}

impl Mesh {
    pub fn to_positioned<T: MeshInstanceInput>(
        self,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        double_style: bool,
    ) -> PositionedMesh<T> {
        PositionedMesh::new(
            self,
            spatial_rx,
            double_style,
        )
    }
}

impl<T: MeshInstanceInput> PositionedMesh<T> {
    pub fn new(
        mesh: Mesh,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        double_style: bool,
    ) -> Self {
        Self {
            mesh,
            instance_buffer: InstanceBuffer::default(),
            attrs: vec![],
            cs_offset: Vector3::new(0.0, 0.0, 0.0),
            double_style,
            spatial_rx,
            original_spatial_data: SpatialData::new(),
        }
    }

    pub fn update(&mut self, global_context: &mut GlobalContext) {
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
                &vec![(Vector3::new(0.0, 0.0, 0.0), 1.0)],
                &self.original_spatial_data,
                self.double_style,
            );
            self.instance_buffer.update(
                "PositionedInstanceBuffer",
                global_context.device(),
                global_context.queue(),
                &self.attrs,
            );
        }
    }

    pub fn render(&mut self, render_pass: &mut RenderPass) {
        self.mesh.render_instanced(1, render_pass, &self.instance_buffer);
    }
}
