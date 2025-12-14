use wgpu::{VertexAttribute, VertexBufferLayout};

pub trait VertexAttrib {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShapeVertex {
    pub position: [f32; 3],
    pub normals: [f32; 3],
    pub dist: f32,
    pub style_index: u32,
}

impl VertexAttrib for ShapeVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: &[VertexAttribute; 4] =
            &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32, 3 => Uint32];
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexNormal {
    pub(crate) position: [f32; 3],
    pub(crate) normals: [f32; 3],
}

impl VertexAttrib for VertexNormal {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: &[VertexAttribute; 2] =
            &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstancePos {
    pub(crate) position: [f32; 3],
    pub(crate) color_alpha: f32,
    pub(crate) matrix: [[f32; 4]; 4],
    pub(crate) bbox: [f32; 4],
    pub(crate) normal_scale: f32,
}

impl VertexAttrib for InstancePos {
    fn desc() -> VertexBufferLayout<'static> {
        const ATTRIBUTES: &[VertexAttribute; 8] = &wgpu::vertex_attr_array![
            4 => Float32x3,
            5 => Float32,
            6 => Float32x4,
            7 => Float32x4,
            8 => Float32x4,
            9 => Float32x4,
            10 => Float32x4,
            11 => Float32,
        ];

        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            // It's easy to forget it should be VertexStepMode::Instance!
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: ATTRIBUTES,
        }
    }
}
