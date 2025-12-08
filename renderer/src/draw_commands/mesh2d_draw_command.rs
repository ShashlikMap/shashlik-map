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
    pub outlined: bool,
    pub feature_layer_tag: Option<String>,
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
            0.0,
            spatial_rx, self.outlined,
            self.screen_paths.with_collision,
        );
        if let Some(tag) = self.feature_layer_tag.as_ref() {
            println!("Mesh2d draw feature layer1: {}", tag);
            if let Some(feature_layer) = layers.feature_layers(tag) {
                println!("Mesh2d draw feature layer2: {}", tag);
                feature_layer.borrow_mut().add_child_with_key(mesh, key);
            }
        } else {
            if self.is_screen {
                layers.screen_shape_layer.borrow_mut().add_child_with_key(mesh, key);
            } else {
                layers.shape_layers(self.real_layer)
                    .borrow_mut()
                    .add_child_with_key(mesh, key);
            }
        }
    }
}
