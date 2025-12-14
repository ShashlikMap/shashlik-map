pub mod mesh2d_draw_command;
pub mod mesh3d_draw_command;
pub mod text_draw_command;

use crate::layers::Layers;
use crate::mesh::mesh::Mesh;
use crate::modifier::render_modifier::SpatialData;
use bytemuck::NoUninit;
use lyon::lyon_tessellation::{LineJoin, VertexBuffers};
use std::ops::Range;
use lyon::path::LineCap;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::Device;

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
}

impl Default for PolylineOptions {
    fn default() -> Self {
        PolylineOptions {
            width: 1f32,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
        }
    }
}

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
        device: &wgpu::Device,
        layers: &mut Layers,
    ) {
        self.draw_commands.iter_mut().for_each(|command| {
            command.execute(
                device,
                self.key.clone(),
                self.spatial_data.clone(),
                self.spatial_tx.subscribe(),
                layers
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
        device: &wgpu::Device,
        key: String,
        spatial_data: SpatialData,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        layers: &mut Layers
    );
}

pub fn geometry_to_mesh<T: NoUninit>(device: &Device, geometry: &VertexBuffers<T, u32>) -> Mesh {
    geometry_to_mesh_with_layers(device, geometry, vec![0..geometry.indices.len()])
}
fn geometry_to_mesh_with_layers<T: NoUninit>(
    device: &Device,
    geometry: &VertexBuffers<T, u32>,
    layers_indices: Vec<Range<usize>>,
) -> Mesh {
    let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(geometry.vertices.as_slice()),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(geometry.indices.as_slice()),
        usage: wgpu::BufferUsages::INDEX,
    });
    let num_indices = geometry.indices.len() as u32;

    Mesh::new(
        vec![vertex_buffer],
        vec![(index_buffer, num_indices as usize)],
        layers_indices,
    )
}
