use crate::draw_commands::DrawCommand;
use crate::geometry_data::TextData;
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::scene_tree::SceneTree;
use crate::nodes::text_node::TextNode;
use std::cell::RefMut;
use wgpu::Device;

#[derive(Clone)]
pub(crate) struct TextDrawCommand {
    pub data: Vec<TextData>,
}

impl DrawCommand for TextDrawCommand {
    fn execute(
        &self,
        _device: &Device,
        key: String,
        spatial_data: SpatialData,
        _spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        _shape_layer: &mut RefMut<SceneTree>,
        _screen_shape_layer: &mut RefMut<SceneTree>,
        _mesh_layer: &mut RefMut<SceneTree>,
        text_layer: &mut RefMut<SceneTree>,
    ) {
        let text_node = TextNode::new(self.data.clone(), spatial_data); // replace?
        text_layer.add_child_with_key(text_node, key.clone());
    }
}
