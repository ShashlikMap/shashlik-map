use crate::canvas_api::MeshInfo;
use crate::draw_commands::{DrawCommand, geometry_to_mesh_with_layers};
use crate::layers::Layers;
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
        _spatial_data: SpatialData,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        layers: &mut Layers,
    ) {
        let mesh =
            geometry_to_mesh_with_layers(&device, &self.mesh, mem::take(&mut self.layers_indices));
        if let Some(feature_layer) = self
            .feature_layer_tag
            .as_ref()
            .and_then(|tag| layers.feature_layers(tag))
        {
            feature_layer.add(
                key,
                device,
                Some(mem::take(&mut self.mesh_info.instance_positions)),
                spatial_rx,
                true,
                false,
                mesh,
            )
        } else {
            if self.is_screen {
                layers.new_screen_shape_layer.add(
                    key,
                    device,
                    Some(mem::take(&mut self.mesh_info.instance_positions)),
                    spatial_rx,
                    !self.is_screen,
                    self.mesh_info.with_collision,
                    mesh,
                );
            } else {
                layers.new_shape_layer.add(
                    key,
                    device,
                    Some(mem::take(&mut self.mesh_info.instance_positions)),
                    spatial_rx,
                    !self.is_screen,
                    self.mesh_info.with_collision,
                    mesh,
                );
            }
        }
    }
}
