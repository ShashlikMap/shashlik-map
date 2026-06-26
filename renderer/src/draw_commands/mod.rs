pub mod mesh2d_draw_command;
pub mod mesh3d_draw_command;
pub mod text_draw_command;

use crate::mesh_layers::layers::Layers;
use crate::modifier::render_modifier::SpatialData;
use lyon::lyon_tessellation::LineJoin;
use lyon::path::LineCap;
use crate::buffer_pool::BufferPool;
use crate::geometry_data::ShapeData;
use crate::global_context::GlobalContext;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normals: [f32; 3],
}

#[derive(Clone, Copy)]
pub enum GeometryType {
    Polyline(PolylineOptions),
    Polygon,
}

#[derive(Clone, Copy)]
pub struct PolylineOptions {
    pub width: f32,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
    pub tolerance: f32,
}

impl Default for PolylineOptions {
    fn default() -> Self {
        PolylineOptions {
            width: 1f32,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            tolerance: 1f32,
        }
    }
}

pub(crate) struct DrawCommands {
    key: String,
    spatial_data: SpatialData,
    spatial_tx: tokio::sync::broadcast::Sender<SpatialData>,
    draw_commands: Vec<Box<dyn DrawCommand>>,
}

pub(crate) struct DrawCommands2 {
    pub(crate) key: String,
    pub(crate) spatial_data: SpatialData,
    pub(crate) spatial_tx: tokio::sync::broadcast::Sender<SpatialData>,
    pub(crate) shapes: Vec<ShapeData>,
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
