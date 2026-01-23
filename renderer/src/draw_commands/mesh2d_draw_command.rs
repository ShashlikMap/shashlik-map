use crate::canvas_api::MeshInfo;
use crate::draw_commands::{DrawCommand, geometry_to_mesh_with_layers};
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
        // TODO Mesh bufs still need to created all the time even though it makes no sense for 
        //  screen_shape_layer. Should it be moved anywhere else?
        let mesh =
            geometry_to_mesh_with_layers(&device, &self.mesh, mem::take(&mut self.layers_indices));

        if let Some(feature_layer) = self
            .feature_layer_tag
            .as_ref()
            .and_then(|tag| layers.feature_layers(tag))
        {
            feature_layer.add(
                key,
                mem::take(&mut self.mesh_info.instance_positions),
                spatial_rx,
                !self.is_screen,
                mesh,
            );
        } else if self.is_screen {
            // TODO check mesh collision
            layers.screen_shape_layer.submit(key,
                                             self.mesh_info.instance_key.as_str(),
                                             mem::take(&mut self.mesh_info.instance_positions).unwrap_or_default(), spatial_data, mesh);
        } else {
            layers.shape_layer.add(
                key,
                mem::take(&mut self.mesh_info.instance_positions),
                spatial_rx,
                !self.is_screen,
                mesh,
            );
        };
    }
}
