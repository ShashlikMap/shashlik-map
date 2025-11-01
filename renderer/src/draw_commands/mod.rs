pub mod mesh2d_draw_command;
pub mod mesh3d_draw_command;
pub mod text_draw_command;

use crate::mesh::mesh::Mesh;
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::scene_tree::SceneTree;
use bytemuck::NoUninit;
use lyon::lyon_tessellation::VertexBuffers;
use std::cell::RefMut;
use wgpu::Device;
use wgpu::util::{BufferInitDescriptor, DeviceExt};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normals: [f32; 3],
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum GeometryType {
    Polyline,
    Polygon,
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
        &self,
        device: &wgpu::Device,
        shape_layer: &mut RefMut<SceneTree>,
        screen_shape_layer: &mut RefMut<SceneTree>,
        mesh_layer: &mut RefMut<SceneTree>,
        text_layer: &mut RefMut<SceneTree>,
    ) {
        self.draw_commands.iter().for_each(|d| {
            d.execute(
                device,
                self.key.clone(),
                self.spatial_data.clone(),
                self.spatial_tx.subscribe(),
                shape_layer,
                screen_shape_layer,
                mesh_layer,
                text_layer
            )
        });
        if self.spatial_tx.receiver_count() > 0 {
            self.spatial_tx.send(self.spatial_data.clone()).unwrap();
        }
    }
}

pub(crate) trait DrawCommand: Send {
    fn execute(
        &self,
        device: &wgpu::Device,
        key: String,
        spatial_data: SpatialData,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        shape_layer: &mut RefMut<SceneTree>,
        screen_shape_layer: &mut RefMut<SceneTree>,
        mesh_layer: &mut RefMut<SceneTree>,
        text_layer: &mut RefMut<SceneTree>,
    );
}

fn geometry_to_mesh<T: NoUninit>(device: &Device, geometry: &VertexBuffers<T, u32>) -> Mesh {
    geometry_to_mesh_with_layers(device, geometry, vec![geometry.indices.len()])
}
fn geometry_to_mesh_with_layers<T: NoUninit>(
    device: &Device,
    geometry: &VertexBuffers<T, u32>,
    layers_indices: Vec<usize>,
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
