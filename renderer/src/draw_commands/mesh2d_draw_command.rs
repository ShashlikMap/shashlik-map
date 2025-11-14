use crate::canvas_api::ScreenPaths;
use crate::draw_commands::{geometry_to_mesh_with_layers, DrawCommand};
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::scene_tree::SceneTree;
use crate::nodes::shape_layers::ShapeLayers;
use crate::vertex_attrs::ShapeVertex;
use lyon::tessellation::VertexBuffers;
use std::cell::RefMut;
use std::ops::Range;

#[derive(Clone)]
pub(crate) struct Mesh2dDrawCommand {
    pub mesh: VertexBuffers<ShapeVertex, u32>,
    pub real_layer: usize,
    pub layers_indices: Vec<Range<usize>>,
    pub screen_paths: ScreenPaths,
    pub is_screen: bool,
}

impl DrawCommand for Mesh2dDrawCommand {
    fn execute(
        &self,
        device: &wgpu::Device,
        key: String,
        _spatial_data: SpatialData,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        shape_layers: &mut ShapeLayers,
        screen_shape_layer: &mut RefMut<SceneTree>,
        _mesh_layer: &mut RefMut<SceneTree>,
        _text_layer: &mut RefMut<SceneTree>,
    ) {
        // TODO remove clone
        let mesh = geometry_to_mesh_with_layers(&device, &self.mesh, self.layers_indices.clone());
        let mesh = mesh.to_positioned_with_instances(
            device,
            self.screen_paths.positions.clone(), // mem::replace
            spatial_rx, true,
            self.screen_paths.with_collision,
        );
        if self.is_screen {
            screen_shape_layer.add_child_with_key(mesh, key);
        } else {
            shape_layers
                .get_shape_layer(self.real_layer)
                .borrow_mut()
                .add_child_with_key(mesh, key);
        }
    }
}
