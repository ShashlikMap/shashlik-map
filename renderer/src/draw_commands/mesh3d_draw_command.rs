use crate::draw_commands::{DrawCommand, MeshVertex};
use crate::mesh::mesh::Mesh;
use crate::mesh_layers::layers::Layers;
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
        let mesh = Mesh::create(&device, &self.mesh);
        layers.mesh_layer.add(key, None, spatial_rx, false, mesh);
    }
}
