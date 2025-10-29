use crate::mesh::mesh::Mesh;
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::scene_tree::SceneTree;
use crate::vertex_attrs::ShapeVertex;
use bytemuck::NoUninit;
use cgmath::Vector3;
use lyon::lyon_tessellation::VertexBuffers;
use std::cell::RefMut;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::Device;

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
    pub fn new(key: String,
               spatial_data: SpatialData,
               spatial_tx: tokio::sync::broadcast::Sender<SpatialData>,
               draw_commands: Vec<Box<dyn DrawCommand>>) -> Self {
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
    ) {
        self.draw_commands.iter().for_each(|d| {
            d.execute(
                device,
                self.key.clone(),
                self.spatial_tx.subscribe(),
                shape_layer,
                screen_shape_layer,
                mesh_layer,
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
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        shape_layer: &mut RefMut<SceneTree>,
        screen_shape_layer: &mut RefMut<SceneTree>,
        mesh_layer: &mut RefMut<SceneTree>,
    );
}

#[derive(Clone)]
pub(crate) struct Mesh2dDrawCommand {
    pub mesh: VertexBuffers<ShapeVertex, u32>,
    pub positions: Vec<Vector3<f32>>,
    pub is_screen: bool,
}

#[derive(Clone)]
pub(crate) struct Mesh3dDrawCommand {
    pub mesh: VertexBuffers<MeshVertex, u32>,
}

impl DrawCommand for Mesh2dDrawCommand {
    fn execute(
        &self,
        device: &wgpu::Device,
        key: String,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        shape_layer: &mut RefMut<SceneTree>,
        screen_shape_layer: &mut RefMut<SceneTree>,
        _mesh_layer: &mut RefMut<SceneTree>,
    ) {

        let mesh = geometry_to_mesh(&device, &self.mesh);
        let mesh = mesh.to_positioned_with_instances(
            device,
            self.positions.clone(), // mem::replace
            spatial_rx,
            true,
        );
        if self.is_screen {
            screen_shape_layer.add_child_with_key(mesh, key);
        } else {
            shape_layer.add_child_with_key(mesh, key);
        }
    }
}

impl DrawCommand for Mesh3dDrawCommand {
    fn execute(
        &self,
        device: &wgpu::Device,
        key: String,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        _shape_layer: &mut RefMut<SceneTree>,
        _screen_shape_layer: &mut RefMut<SceneTree>,
        mesh_layer: &mut RefMut<SceneTree>,
    ) {
        let mesh = geometry_to_mesh(&device, &self.mesh);
        mesh_layer.add_child_with_key(mesh.to_positioned(device, spatial_rx), key.clone());
    }
}
fn geometry_to_mesh<T: NoUninit>(device: &Device, geometry: &VertexBuffers<T, u32>) -> Mesh {
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
    )
}
