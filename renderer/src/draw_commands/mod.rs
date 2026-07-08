pub mod mesh2d_draw_command;
pub mod mesh3d_draw_command;
pub mod text_draw_command;

use crate::buffer_pool::BufferPool;
use crate::global_context::GlobalContext;
use crate::mesh_layers::layers::Layers;
use wgpu_canvas::render_modifier::SpatialData;

pub(crate) struct DrawCommands {
    key: String,
    spatial_data: SpatialData,
    spatial_tx: tokio::sync::broadcast::Sender<SpatialData>,
    draw_commands: Vec<Box<dyn DrawCommand>>,
}

impl DrawCommands {
    pub fn new(
        key: String,
        spatial_data: SpatialData,
        spatial_tx: tokio::sync::broadcast::Sender<SpatialData>,
        draw_commands: Vec<Box<dyn DrawCommand>>,
    ) -> Self {
        DrawCommands {
            key,
            spatial_data,
            spatial_tx,
            draw_commands,
        }
    }
    pub(crate) fn execute(
        &mut self,
        global_context: &mut GlobalContext,
        layers: &mut Layers,
        buffer_pool: &mut BufferPool
    ) {
        self.draw_commands.iter_mut().for_each(|command| {
            command.execute(
                global_context,
                self.key.clone(),
                self.spatial_data.clone(),
                self.spatial_tx.subscribe(),
                layers,
                buffer_pool
            )
        });
        if self.spatial_tx.receiver_count() > 0 {
            self.spatial_tx.send(self.spatial_data.clone()).unwrap();
        }
    }
}

pub(crate) trait DrawCommand: Send {
    fn execute(
        &mut self,
        global_context: &mut GlobalContext,
        key: String,
        spatial_data: SpatialData,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        layers: &mut Layers,
        buffer_pool: &mut BufferPool
    );
}
