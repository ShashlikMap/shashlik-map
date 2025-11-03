use crate::draw_commands::{DrawCommand, geometry_to_mesh_with_layers};
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::scene_tree::SceneTree;
use crate::vertex_attrs::ShapeVertex;
use cgmath::Vector3;
use lyon::tessellation::VertexBuffers;
use std::cell::RefMut;

#[derive(Clone)]
pub(crate) struct Mesh2dDrawCommand {
    pub mesh: VertexBuffers<ShapeVertex, u32>,
    pub layers_indices: Vec<usize>,
    pub positions: Vec<Vector3<f32>>,
    pub is_screen: bool,
}

impl DrawCommand for Mesh2dDrawCommand {
    fn execute(
        &self,
        device: &wgpu::Device,
        key: String,
        _spatial_data: SpatialData,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        shape_layer: &mut RefMut<SceneTree>,
        screen_shape_layer: &mut RefMut<SceneTree>,
        _mesh_layer: &mut RefMut<SceneTree>,
        _text_layer: &mut RefMut<SceneTree>,
    ) {
        // TODO remove clone
        let mesh = geometry_to_mesh_with_layers(&device, &self.mesh, self.layers_indices.clone());
        let mesh = mesh.to_positioned_with_instances(
            device,
            self.positions.clone(), // mem::replace
            spatial_rx,
            true,
            self.is_screen,
        );
        if self.is_screen {
            screen_shape_layer.add_child_with_key(mesh, key);
        } else {
            shape_layer.add_child_with_key(mesh, key);
        }
    }
}
