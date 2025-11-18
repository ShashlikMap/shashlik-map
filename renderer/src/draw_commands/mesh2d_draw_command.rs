use crate::canvas_api::ScreenPaths;
use crate::draw_commands::{geometry_to_mesh_with_layers, DrawCommand};
use crate::layers::Layers;
use crate::modifier::render_modifier::SpatialData;
use crate::vertex_attrs::ShapeVertex;
use lyon::tessellation::VertexBuffers;
use std::mem;
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
        &mut self,
        device: &wgpu::Device,
        key: String,
        _spatial_data: SpatialData,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        layers: &mut Layers,
    ) {
        let mesh = geometry_to_mesh_with_layers(&device, &self.mesh, mem::take(&mut self.layers_indices));

        let mesh = mesh.to_positioned_with_instances(
            device,
            mem::take(&mut self.screen_paths.positions),
            spatial_rx, true,
            self.screen_paths.with_collision,
        );
        if self.is_screen {
            layers.screen_shape_layer.borrow_mut().add_child_with_key(mesh, key);
        } else {
            layers.shape_layers(self.real_layer)
                .borrow_mut()
                .add_child_with_key(mesh, key);
        }
    }
}
