use crate::draw_commands::{DrawCommand};
use crate::mesh::mesh::{Mesh};
use crate::mesh_layers::layers::Layers;
use wgpu_canvas::render_modifier::SpatialData;
use lyon::lyon_tessellation::VertexBuffers;
use wgpu_canvas::geometry_data::{MeshVertex, StyledRangeInfo};
use crate::buffer_pool::BufferPool;
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
        buffer_pool: &mut BufferPool
    ) {

        let mesh = Mesh::create(Some(key.as_str()), global_context, buffer_pool, &self.mesh, StyledRangeInfo(0, ""));
        layers.mesh_layer.add(key, spatial_rx, false, mesh);
    }
}
