use crate::draw_commands::DrawCommand;
use crate::geometry_data::TextData;
use crate::layers::Layers;
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::text_node::TextNode;
use std::mem;
use wgpu::Device;

#[derive(Clone)]
pub(crate) struct TextDrawCommand {
    pub data: Vec<TextData>,
}

impl DrawCommand for TextDrawCommand {
    fn execute(
        &mut self,
        _device: &Device,
        key: String,
        spatial_data: SpatialData,
        _spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        layers: &mut Layers,
    ) {
        let text_node = TextNode::new(mem::take(&mut self.data), spatial_data);
        layers.text_layer.borrow_mut().add_child_with_key(text_node, key.clone());
    }
}
