use crate::buffer_pool::BufferPool;
use crate::global_context::GlobalContext;
use crate::mesh::InstanceBuffer;
use crate::vertex_attrs::GeneralInstanceInput;
use glam::Mat4;
use lyon::lyon_tessellation::VertexBuffers;
use renderer_common::geometry_data::MeshVertex;
use wgpu::{Buffer, RenderPass};

pub(crate) struct VirtualGroundMesh {
    vertices: Buffer,
    instance_buffer: Buffer,
}

impl VirtualGroundMesh {
    pub fn new(global_context: &GlobalContext, buffer_pool: &mut BufferPool) -> Self {
        let mut geometry_buffer: VertexBuffers<MeshVertex, u32> = VertexBuffers::new();
        geometry_buffer.vertices.push(MeshVertex {
            position: [-1.0, 1.0, 0.0],
            normals: [0.0, 0.0, 1.0],
        });
        geometry_buffer.vertices.push(MeshVertex {
            position: [3.0, 1.0, 0.0],
            normals: [0.0, 0.0, 1.0],
        });
        geometry_buffer.vertices.push(MeshVertex {
            position: [-1.0, -3.0, 0.0],
            normals: [0.0, 0.0, 1.0],
        });
        let vertices = buffer_pool.create(
            global_context.device(),
            global_context.queue(),
            None,
            "GroundVertexBuffer",
            wgpu::BufferUsages::VERTEX,
            geometry_buffer.vertices.as_slice(),
        );

        let mut instance_buffer = InstanceBuffer::default();
        instance_buffer.update(
            "VirtualGroundInstanceBuffer",
            global_context,
            &vec![GeneralInstanceInput {
                position: [0.0, 0.0, 0.0],
                color_alpha: 1.0,
                matrix: Mat4::IDENTITY.to_cols_array_2d(),
                ortho_transform: 1
            }],
        );

        let instance_buffer = instance_buffer
            .buffer_with_id
            .as_ref()
            .expect("virtual instance buffer should exist")
            .buffer()
            .clone();
        Self {
            vertices,
            instance_buffer,
        }
    }
    pub fn render(&mut self, render_pass: &mut RenderPass) {
        render_pass.set_vertex_buffer(0, self.vertices.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.draw(0..3, 0..1);
    }
}
