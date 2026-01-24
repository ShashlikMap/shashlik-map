use crate::canvas_api::MeshInfo;
use crate::draw_commands::DrawCommand;
use crate::mesh::mesh::Mesh;
use crate::mesh_layers::layers::Layers;
use crate::modifier::render_modifier::SpatialData;
use crate::vertex_attrs::ShapeVertex;
use lyon::tessellation::VertexBuffers;
use std::mem;
use std::ops::Range;

#[derive(Clone)]
pub(crate) struct Mesh2dDrawCommand {
    pub mesh: VertexBuffers<ShapeVertex, u32>,
    pub layers_indices: Vec<Range<usize>>,
    pub mesh_info: MeshInfo,
    pub is_screen: bool,
    pub feature_layer_tag: Option<String>,
}

impl DrawCommand for Mesh2dDrawCommand {
    fn execute(
        &mut self,
        device: &wgpu::Device,
        key: String,
        spatial_data: SpatialData,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        layers: &mut Layers,
    ) {
        if let Some(feature_layer) = self
            .feature_layer_tag
            .as_ref()
            .and_then(|tag| layers.feature_layers(tag))
        {
            let mesh =
                Mesh::create_layered(&device, &self.mesh, mem::take(&mut self.layers_indices));
            feature_layer.add(
                key,
                spatial_rx,
                !self.is_screen,
                mesh,
            );
        } else if self.is_screen {
            layers
                .screen_shape_layer
                .submit(key.as_str(), spatial_data, device, self);
        } else {
            let mesh =
                Mesh::create_layered(&device, &self.mesh, mem::take(&mut self.layers_indices));
            layers.shape_layer.add(
                key,
                spatial_rx,
                !self.is_screen,
                mesh,
            );
        };
    }
}
