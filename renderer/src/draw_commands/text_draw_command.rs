use crate::draw_commands::DrawCommand;
use crate::geometry_data::TextData;
use crate::global_context::GlobalContext;
use crate::mesh_layers::layers::{Layers, WORLD_TEXT_LAYER};
use crate::modifier::render_modifier::SpatialData;
use std::mem;

pub(crate) struct TextDrawCommand {
    pub data: Vec<TextData>,
}

impl DrawCommand for TextDrawCommand {
    fn execute(
        &mut self,
        _global_context: &mut GlobalContext,
        key: String,
        spatial_data: SpatialData,
        _spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        layers: &mut Layers,
    ) {
        layers
            .text_feature_layers.get_layer(WORLD_TEXT_LAYER).unwrap()
            .add(key, mem::take(&mut self.data), spatial_data);
    }
}
