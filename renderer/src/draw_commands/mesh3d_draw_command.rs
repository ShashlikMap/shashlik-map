use crate::draw_commands::{DrawCommand, MeshVertex, geometry_to_mesh};
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::scene_tree::SceneTree;
use lyon::lyon_tessellation::VertexBuffers;
use std::cell::RefMut;

#[derive(Clone)]
pub(crate) struct Mesh3dDrawCommand {
    pub mesh: VertexBuffers<MeshVertex, u32>,
}

impl DrawCommand for Mesh3dDrawCommand {
    fn execute(
        &self,
        device: &wgpu::Device,
        key: String,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        _shape_layer: &mut RefMut<SceneTree>,
        _screen_shape_layer: &mut RefMut<SceneTree>,
        mesh_layer: &mut RefMut<SceneTree>,
    ) {
        let mesh = geometry_to_mesh(&device, &self.mesh);
        mesh_layer.add_child_with_key(mesh.to_positioned(device, spatial_rx), key.clone());
    }
}
