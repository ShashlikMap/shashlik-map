use crate::buffer_pool::BufferPool;
use crate::draw_commands::DrawCommand;
use crate::global_context::GlobalContext;
use crate::mesh_layers::layers::Layers;
use log::error;
use renderer_common::render_modifier::SpatialData;

pub(crate) struct RasterDrawCommand {
    /// premultiplied alpha RGBA8
    pub rgba: Vec<u8>,
    pub size: (u32, u32),
    pub world_size: f32,
}

impl RasterDrawCommand {
    /// Decodes an image (PNG etc.) on the caller(background) thread.
    /// Returns None for fully transparent images, there is nothing to draw.
    pub fn decode(encoded: &[u8], world_size: f32) -> Option<Self> {
        let image = image::load_from_memory(encoded)
            .inspect_err(|e| error!("Can't decode raster: {e}"))
            .ok()?
            .to_rgba8();
        let size = image.dimensions();
        let mut rgba = image.into_raw();
        let mut visible = false;
        rgba.chunks_exact_mut(4).for_each(|pixel| {
            let alpha = pixel[3] as u16;
            if alpha > 0 {
                visible = true;
            }
            for channel in &mut pixel[..3] {
                *channel = ((*channel as u16 * alpha + 127) / 255) as u8;
            }
        });
        visible.then_some(Self { rgba, size, world_size })
    }
}

impl DrawCommand for RasterDrawCommand {
    fn execute(
        &mut self,
        global_context: &mut GlobalContext,
        key: String,
        spatial_data: SpatialData,
        _spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        layers: &mut Layers,
        _buffer_pool: &mut BufferPool,
    ) {
        layers.raster_layer.add(
            key,
            spatial_data.transform,
            self.world_size,
            self.size,
            &self.rgba,
            global_context,
        );
    }
}
