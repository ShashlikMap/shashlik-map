use crate::draw_commands::{geometry_to_mesh, DrawCommand, MeshVertex};
use crate::layers::Layers;
use crate::modifier::render_modifier::SpatialData;
use cgmath::{Vector2, Vector3};
use lyon::lyon_tessellation::VertexBuffers;
use wgpu::Device;

#[derive(Clone)]
pub(crate) struct TextDrawCommand2 {
    pub glyph_mesh: VertexBuffers<MeshVertex, u32>,
    pub offset: Vector2<f32>,
    pub rotation: f32,
}

impl DrawCommand for TextDrawCommand2 {
    fn execute(
        &mut self,
        device: &Device,
        key: String,
        _spatial_data: SpatialData,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        layers: &mut Layers,
    ) {
        let mesh = geometry_to_mesh(&device, &self.glyph_mesh);
        let mesh = mesh.to_positioned_with_instances(
            device,
            vec![Vector3::new(self.offset.x as f64, self.offset.y as f64, 0.0)],
            self.rotation,
            spatial_rx,
            false,
            false,
        );
        layers
            .mesh_layer
            .borrow_mut()
            .add_child_with_key(mesh, key.clone());
    }
}
