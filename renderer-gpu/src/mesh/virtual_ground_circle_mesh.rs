use crate::buffer_pool::BufferPool;
use crate::global_context::GlobalContext;
use crate::mesh::InstanceBuffer;
use crate::mesh::mesh::Mesh;
use crate::vertex_attrs::GeneralInstanceInput;
use glam::Mat4;
use lyon::lyon_tessellation::VertexBuffers;
use renderer_common::geometry_data::{MeshVertex, StyledRangeInfo};
use wgpu::{Buffer, RenderPass};

pub(crate) struct VirtualGroundCircleMesh {
    mesh: Mesh,
    instance_buffer: Buffer,
}

impl VirtualGroundCircleMesh {
    pub fn new(global_context: &GlobalContext, buffer_pool: &mut BufferPool) -> Self {
        let mut geometry_buffer: VertexBuffers<MeshVertex, u32> = VertexBuffers::new();
        geometry_buffer.vertices.push(MeshVertex {
            position: [0.000, 1.082, 0.0],
            normals: [0.0, 0.0, 1.0],
        });
        geometry_buffer.vertices.push(MeshVertex {
            position: [0.765, 0.765, 0.0],
            normals: [0.0, 0.0, 1.0],
        });
        geometry_buffer.vertices.push(MeshVertex {
            position: [1.082, 0.000, 0.0],
            normals: [0.0, 0.0, 1.0],
        });
        geometry_buffer.vertices.push(MeshVertex {
            position: [0.765, -0.765, 0.0],
            normals: [0.0, 0.0, 1.0],
        });
        geometry_buffer.vertices.push(MeshVertex {
            position: [0.000, -1.082, 0.0],
            normals: [0.0, 0.0, 1.0],
        });
        geometry_buffer.vertices.push(MeshVertex {
            position: [-0.765, -0.765, 0.0],
            normals: [0.0, 0.0, 1.0],
        });
        geometry_buffer.vertices.push(MeshVertex {
            position: [-1.082, 0.000, 0.0],
            normals: [0.0, 0.0, 1.0],
        });
        geometry_buffer.vertices.push(MeshVertex {
            position: [-0.765, 0.765, 0.0],
            normals: [0.0, 0.0, 1.0],
        });

        geometry_buffer.vertices.push(MeshVertex {
            position: [0.000, 0.000, 0.0],
            normals: [0.0, 0.0, 1.0],
        });

        geometry_buffer.indices.extend_from_slice(&[
            8, 0, 1, 8, 1, 2, 8, 2, 3, 8, 3, 4, 8, 4, 5, 8, 5, 6, 8, 6, 7, 8, 7, 0,
        ]);

        let mesh = Mesh::create(
            None,
            global_context,
            buffer_pool,
            &geometry_buffer,
            StyledRangeInfo::default(),
        );

        let mut instance_buffer = InstanceBuffer::default();
        instance_buffer.update(
            "VirtualGroundCircleMeshInstanceBuffer",
            global_context,
            &vec![GeneralInstanceInput {
                position: [0.0, 0.0, 0.0],
                color_alpha: 1.0,
                matrix: Mat4::IDENTITY.to_cols_array_2d(),
                virtual_plane: 1,
            }],
        );

        let instance_buffer = instance_buffer
            .buffer_with_id
            .as_ref()
            .expect("virtual ground circle instance buffer should exist")
            .buffer()
            .clone();
        Self {
            mesh,
            instance_buffer,
        }
    }
    pub fn render(&mut self, render_pass: &mut RenderPass) {
        render_pass.set_vertex_buffer(0, self.mesh.vertex_buf.slice(..));
        render_pass.set_index_buffer(self.mesh.index_buf.0.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.draw_indexed(0u32..self.mesh.index_buf.1 as u32, 0, 0..1);
    }
}
