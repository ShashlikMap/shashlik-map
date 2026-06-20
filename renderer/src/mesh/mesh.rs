use crate::mesh::InstanceBuffer;
use crate::vertex_attrs::MeshVertexWithUV;
use bytemuck::{NoUninit, Pod};
use log::error;
use lyon::lyon_tessellation::VertexBuffers;
use std::ops::Range;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{Buffer, Device, RenderPass};

#[derive(Clone)]
pub struct StyledRangeInfo(pub u8, pub &'static str);
#[derive(Clone)]
pub struct StyledRange(pub Range<usize>, pub StyledRangeInfo);

pub struct Mesh {
    pub vertex_buf: Buffer,
    pub index_buf: (Buffer, usize),
    pub layers_indices: Vec<StyledRange>,
}

impl Mesh {
    pub fn new(
        v_buf: Buffer,
        i_buf: (Buffer, usize),
        layers_indices: Vec<StyledRange>,
    ) -> Self {
        Self {
            vertex_buf: v_buf,
            index_buf: i_buf,
            layers_indices,
        }
    }

    pub fn quad(device: &Device, width: f32, height: f32) -> Self {
        let mut geometry_buffer: VertexBuffers<MeshVertexWithUV, u32> = VertexBuffers::new();
        geometry_buffer.vertices.push(MeshVertexWithUV {
            position: [0.0, 0.0],
            color: [0.0, 0.0, 0.0],
            uv: [0.0, 1.0],
        });
        geometry_buffer.vertices.push(MeshVertexWithUV {
            position: [width, 0.0],
            color: [0.0, 0.0, 0.0],
            uv: [1.0, 1.0],
        });
        geometry_buffer.vertices.push(MeshVertexWithUV {
            position: [0.0, height],
            color: [0.0, 0.0, 0.0],
            uv: [0.0, 0.0],
        });
        geometry_buffer.vertices.push(MeshVertexWithUV {
            position: [width, height],
            color: [0.0, 0.0, 0.0],
            uv: [1.0, 0.0],
        });

        geometry_buffer.indices.push(0);
        geometry_buffer.indices.push(2);
        geometry_buffer.indices.push(3);

        geometry_buffer.indices.push(1);
        geometry_buffer.indices.push(0);
        geometry_buffer.indices.push(3);
        Self::create(device, &geometry_buffer, StyledRangeInfo(0, ""))
    }

    pub fn create<T: NoUninit>(device: &Device, geometry: &VertexBuffers<T, u32>, styled_range_info: StyledRangeInfo) -> Self {
        Self::create_layered(device, geometry, vec![StyledRange(0..geometry.indices.len(), styled_range_info)])
    }

    pub fn create_layered<T: NoUninit>(
        device: &Device,
        geometry: &VertexBuffers<T, u32>,
        layers_indices: Vec<StyledRange>,
    ) -> Self {
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
            vertex_buffer,
            (index_buffer, num_indices as usize),
            layers_indices,
        )
    }

    pub fn render_instanced<T: Pod>(
        &self,
        slot: Option<u32>,
        render_pass: &mut RenderPass,
        instance_buffer: &InstanceBuffer<T>,
        disable_skip_mesh_feature: bool,
        indirect_args: Option<&Buffer>
    ) {
        if instance_buffer.length > 0
            && let Some(buffer) = instance_buffer.buffer.as_ref()
        {
            if let Some(slot) = slot {
                render_pass.set_vertex_buffer(slot, buffer.slice(..));
            }
            let range = 0..instance_buffer.length as u32;
            self.render(render_pass, &range, disable_skip_mesh_feature, indirect_args);
        }
    }

    fn render(&self, render_pass: &mut RenderPass, instances: &Range<u32>, disable_skip_mesh_feature: bool, indirect_args: Option<&Buffer>) {
        let v_buf = &self.vertex_buf;
        let i_buf = &self.index_buf.0;
        if v_buf.size() > 0 && i_buf.size() > 0 {
            render_pass.set_vertex_buffer(0, v_buf.slice(..));
            render_pass.set_index_buffer(i_buf.slice(..), wgpu::IndexFormat::Uint32);
            for range in &self.layers_indices {
                let styled_range_info = &range.1;
                if disable_skip_mesh_feature && styled_range_info.1 == "skip" {
                    continue;
                }

                if let Some(indirect_args) = indirect_args {
                    render_pass.draw_indexed_indirect(indirect_args, 0);
                } else {
                    let start = range.0.start;
                    let end = range.0.end;

                    // draw instances
                    let instances_range = instances.start + styled_range_info.0 as u32..instances.end;
                    render_pass.draw_indexed(start as u32..end as u32, 0, instances_range);
                }
            }
        } else {
            error!("Vertex/Index buffer are empty");
        }
    }
}
