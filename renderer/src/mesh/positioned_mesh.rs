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
    instance_positions_and_alpha: Vec<(Vector3<f64>, f32)>, // TODO Proper structure with bound
    cs_offset: Vector3<f64>,
    is_two_instances: bool,
    spatial_rx: Receiver<SpatialData>,
    original_spatial_data: SpatialData,
    with_collisions: bool,
}

impl Mesh {
    pub fn to_positioned<T: MeshInstanceInput>(
        self,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
    ) -> PositionedMesh<T> {
        PositionedMesh::new(self, None, spatial_rx, false, false)
    }
    pub fn to_positioned_with_instances<T: MeshInstanceInput>(
        self,
        instance_positions: Option<Vec<Vector3<f64>>>,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        is_two_instances: bool,
        with_collisions: bool,
    ) -> PositionedMesh<T> {
        PositionedMesh::new(
            self,
            instance_positions,
            spatial_rx,
            is_two_instances,
            with_collisions,
        )
    }
}

impl<T: MeshInstanceInput> PositionedMesh<T> {
    pub fn new(
        mesh: Mesh,
        instance_positions: Option<Vec<Vector3<f64>>>,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        is_two_instances: bool,
        with_collisions: bool,
    ) -> Self {
        let instance_positions_and_alpha = instance_positions
            .unwrap_or(vec![Vector3::new(0.0, 0.0, 0.0)])
            .iter()
            .map(|v| (*v, 1.0))
            .collect();
        Self {
            mesh,
            instance_buffer: InstanceBuffer::default(),
            attrs: vec![],
            instance_positions_and_alpha,
            cs_offset: Vector3::new(0.0, 0.0, 0.0),
            is_two_instances,
            spatial_rx,
            original_spatial_data: SpatialData::new(),
            with_collisions,
        }
    }

    pub fn update(&mut self, global_context: &mut GlobalContext) {
        let cs_offset_updated = global_context.view_projection.cs_offset != self.cs_offset;
        self.cs_offset = global_context.view_projection.cs_offset;
        let mut update_attrs = self.with_collisions || cs_offset_updated;

        if let Ok(spatial_data) = self.spatial_rx.no_lagged() {
            self.original_spatial_data = spatial_data;
            update_attrs = true;
        }
        
        if update_attrs {
            T::fill_attrs(
                &mut self.attrs,
                &self.cs_offset,
                &self.instance_positions_and_alpha,
                &self.original_spatial_data,
                self.is_two_instances,
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
        if let Some(buffer) = self.instance_buffer.buffer.as_ref() {
            render_pass.set_vertex_buffer(1, buffer.slice(..));
            let range = 0u32..self.instance_buffer.length as u32;
            self.mesh.render(render_pass, &range);
        }
    }
}
