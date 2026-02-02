use crate::draw_commands::{DrawCommand, MeshVertex};
use crate::mesh::mesh::Mesh;
use crate::mesh_layers::layers::Layers;
use crate::modifier::render_modifier::SpatialData;
use lyon::lyon_tessellation::VertexBuffers;
use crate::global_context::GlobalContext;

#[derive(Clone)]
pub(crate) struct Mesh3dDrawCommand {
    pub mesh: VertexBuffers<MeshVertex, u32>,
}

impl DrawCommand for Mesh3dDrawCommand {
    fn execute(
        &mut self,
        global_context: &mut GlobalContext,
        key: String,
        _spatial_data: SpatialData,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        layers: &mut Layers,
    ) {
        let mesh = Mesh::create(&global_context.device(), &self.mesh, 0);
        layers.mesh_layer.add(key, spatial_rx, false, mesh);
    }
}
