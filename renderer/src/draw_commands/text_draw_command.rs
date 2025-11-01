use crate::draw_commands::DrawCommand;
use crate::geometry_data::TextData;
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::scene_tree::SceneTree;
use std::cell::RefMut;
use wgpu::Device;

#[derive(Clone)]
pub(crate) struct TextDrawCommand {
    pub data: TextData,
}

impl DrawCommand for TextDrawCommand {
    fn execute(
        &self,
        _device: &Device,
        _key: String,
        _spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        _shape_layer: &mut RefMut<SceneTree>,
        _screen_shape_layer: &mut RefMut<SceneTree>,
        _mesh_layer: &mut RefMut<SceneTree>,
    ) {
        println!("TODO TextDrawCommand not implemented yet: execute {:?}",self.data.text);
    }
}
