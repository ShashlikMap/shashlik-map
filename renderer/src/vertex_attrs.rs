use std::mem;
use wgpu::{BufferAddress, VertexAttribute, VertexStepMode};
use crate::draw_commands::MeshVertex;

pub trait VertexAttrib: Sized {
    const ATTRIBUTES: &[VertexAttribute];
    const STEP_MODE: wgpu::VertexStepMode;

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as BufferAddress,
            step_mode: Self::STEP_MODE,
            attributes: Self::ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShapeVertex {
    pub position: [f32; 3],
    pub normals: [f32; 3],
    pub dist: f32,
    pub style_index: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertexWithUV {
    pub mesh_vertex: MeshVertex,
    pub uv: [f32; 3],
}

impl VertexAttrib for MeshVertexWithUV {
    const ATTRIBUTES: &[VertexAttribute] =
        &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3];

    const STEP_MODE: VertexStepMode = wgpu::VertexStepMode::Vertex;
}


impl VertexAttrib for ShapeVertex {
    const ATTRIBUTES: &[VertexAttribute] =
        &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32, 3 => Uint32];

    const STEP_MODE: VertexStepMode = wgpu::VertexStepMode::Vertex;
}

impl VertexAttrib for MeshVertex {
    const ATTRIBUTES: &[VertexAttribute] =
        &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    const STEP_MODE: VertexStepMode = wgpu::VertexStepMode::Vertex;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GeneralInstanceInput {
    pub(crate) position: [f32; 3],
    pub(crate) color_alpha: f32,
    pub(crate) matrix: [[f32; 4]; 4],
}

impl VertexAttrib for GeneralInstanceInput {
    const ATTRIBUTES: &[VertexAttribute] = &wgpu::vertex_attr_array![
        4 => Float32x3,
        5 => Float32,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
        9 => Float32x4,
    ];

    const STEP_MODE: VertexStepMode = wgpu::VertexStepMode::Instance;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShapeInstanceInput {
    pub(crate) position: [f32; 3],
    pub(crate) color_alpha: f32,
    pub(crate) matrix: [[f32; 4]; 4],
    pub(crate) bbox: [f32; 4],
}
impl VertexAttrib for ShapeInstanceInput {
    const ATTRIBUTES: &[VertexAttribute] = &wgpu::vertex_attr_array![
        4 => Float32x3,
        5 => Float32,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
        9 => Float32x4,
        10 => Float32x4,
    ];

    const STEP_MODE: VertexStepMode = wgpu::VertexStepMode::Instance;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextInstanceInput {
    pub(crate) position: [f32; 3],
    pub(crate) color_alpha: f32,
    pub(crate) matrix: [[f32; 4]; 4],
    pub(crate) screen_space: u32,
}
impl VertexAttrib for TextInstanceInput {
    const ATTRIBUTES: &[VertexAttribute] = &wgpu::vertex_attr_array![
        4 => Float32x3,
        5 => Float32,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
        9 => Float32x4,
        12 => Uint32,
    ];

    const STEP_MODE: VertexStepMode = wgpu::VertexStepMode::Instance;
}

