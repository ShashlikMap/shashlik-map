use crate::canvas_api::MeshInfo;
use crate::draw_commands::DrawCommand;
use crate::global_context::GlobalContext;
use crate::mesh::mesh::{Mesh, StyledRange};
use crate::mesh_layers::layers::Layers;
use crate::modifier::render_modifier::SpatialData;
use crate::vertex_attrs::ShapeVertex;
use lyon::tessellation::VertexBuffers;
use std::mem;

#[derive(Clone)]
pub(crate) struct Mesh2dCommandBatch {
    pub mesh: VertexBuffers<ShapeVertex, u32>,
    pub layers_indices: Vec<StyledRange>,
    pub mesh_info: MeshInfo,
}

#[derive(Clone)]
pub(crate) struct Mesh2dDrawCommand {
    pub batches: Vec<Mesh2dCommandBatch>,
    pub is_screen: bool,
    pub feature_layer_tag: Option<String>,
}

impl DrawCommand for Mesh2dDrawCommand {
    fn execute(
        &mut self,
        global_context: &mut GlobalContext,
        key: String,
        spatial_data: SpatialData,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        layers: &mut Layers,
    ) {
        let device = global_context.device();
        if let Some(feature_layer) = self
            .feature_layer_tag
            .as_ref()
            .and_then(|tag| layers.feature_layers(tag))
        {
            if let Some(first_batch) = self.batches.first_mut() {
                let mesh = Mesh::create_layered(
                    &device,
                    &first_batch.mesh,
                    mem::take(&mut first_batch.layers_indices),
                );
                feature_layer.add(key.clone(), spatial_rx, !self.is_screen, mesh);
            }
        } else if self.is_screen {
            layers
                .screen_shape_layer
                .submit(key.as_str(), spatial_data, global_context, self);
        } else {
            if let Some(first_batch) = self.batches.first_mut() {
                let mesh = Mesh::create_layered(
                    &device,
                    &first_batch.mesh,
                    mem::take(&mut first_batch.layers_indices),
                );
                layers
                    .shape_layer
                    .add(key, spatial_rx, !self.is_screen, mesh);
            }
        };
    }
}
