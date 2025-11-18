use crate::draw_commands::{geometry_to_mesh, DrawCommand, MeshVertex};
use crate::layers::Layers;
use crate::modifier::render_modifier::SpatialData;
use lyon::lyon_tessellation::VertexBuffers;

#[derive(Clone)]
pub(crate) struct Mesh3dDrawCommand {
    pub mesh: VertexBuffers<MeshVertex, u32>,
}

impl DrawCommand for Mesh3dDrawCommand {
    fn execute(
        &mut self,
        device: &wgpu::Device,
        key: String,
        _spatial_data: SpatialData,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        layers: &mut Layers,
    ) {
        let mesh = geometry_to_mesh(&device, &self.mesh);
        layers.mesh_layer.borrow_mut().add_child_with_key(mesh.to_positioned(device, spatial_rx), key.clone());
    }
}
