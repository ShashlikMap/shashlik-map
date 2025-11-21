use crate::draw_commands::DrawCommand;
use crate::layers::Layers;
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::text_node::TextNode;
use crate::text::text_renderer::GlyphData;
use std::mem;
use wgpu::Device;

#[derive(Clone)]
pub(crate) struct TextDrawCommand2 {
    pub glyphs: Vec<GlyphData>,
}

impl DrawCommand for TextDrawCommand2 {
    fn execute(
        &mut self,
        _device: &Device,
        key: String,
        spatial_data: SpatialData,
        _spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        layers: &mut Layers,
    ) {
        let text_node = TextNode::new2(mem::take(&mut self.glyphs), spatial_data);
        layers
            .text_layer
            .borrow_mut()
            .add_child_with_key(text_node, key.clone());
    }
}
