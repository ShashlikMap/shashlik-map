use glam::{Vec2, Vec4};
use renderer_common::geometry_data::MeshVertex;
use wgpu::{BufferAddress, VertexAttribute, VertexStepMode};

pub trait VertexAttrib: Sized {
    const ATTRIBUTES: &[VertexAttribute];
    const STEP_MODE: wgpu::VertexStepMode;

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        Self::desc_with_stride(size_of::<Self>() as BufferAddress)
    }

    fn desc_no_stride() -> wgpu::VertexBufferLayout<'static> {
        Self::desc_with_stride(0)
    }

    fn desc_with_stride(array_stride: BufferAddress) -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride,
            step_mode: Self::STEP_MODE,
            attributes: Self::ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShapeVertex {
    pub position: [f32; 2],
    pub normals: [i16; 2],
    pub uv: [u16; 2],
    pub dist: u16,
    pub style_index: u8,
    _pad: u8,
}

impl ShapeVertex {
    pub fn new(position: [f32; 2], normals: [f32; 2], uv: [f32; 2], dist: f32, style_index: u8) -> Self {
        let normals = (Vec2::new(normals[0], normals[1]) * 32767.0).round().as_i16vec2().into();
        let uv = (Vec2::new(uv[0], uv[1]) * 65535.0).round().as_u16vec2().into();
        ShapeVertex {
            position,
            normals,
            uv,
            dist: dist.round() as u16,
            style_index,
            _pad: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertexWithUV {
    pub position: [f32; 2],
    pub color: [u8; 4],
    pub uv: [u16; 2],
}

impl MeshVertexWithUV {
    pub fn new(position: [f32; 2], color: [f32; 4], uv: [f32; 2]) -> Self {
        let color = (Vec4::new(color[0], color[1], color[2], 1.0) * 255.0).round().as_u8vec4().into();
        let uv = (Vec2::new(uv[0], uv[1]) * 65535.0).round().as_u16vec2().into();
        Self {
            position,
            color,
            uv,
        }
    }
}

impl VertexAttrib for MeshVertexWithUV {
    const ATTRIBUTES: &[VertexAttribute] =
        &wgpu::vertex_attr_array![0 => Float32x2, 1 => Unorm8x4, 2 => Unorm16x2];

    const STEP_MODE: VertexStepMode = wgpu::VertexStepMode::Vertex;
}


impl VertexAttrib for ShapeVertex {
    const ATTRIBUTES: &[VertexAttribute] =
        &wgpu::vertex_attr_array![0 => Float32x2, 1 => Snorm16x2, 2 => Unorm16x2, 3 => Uint16, 4 => Uint8];

    const STEP_MODE: VertexStepMode = wgpu::VertexStepMode::Vertex;
}

impl VertexAttrib for MeshVertex {
    const ATTRIBUTES: &[VertexAttribute] =
        &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    const STEP_MODE: VertexStepMode = wgpu::VertexStepMode::Vertex;
}

#[repr(C)]
#[derive(Default, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GeneralInstanceInput {
    pub(crate) position: [f32; 3],
    pub(crate) color_alpha: f32,
    pub(crate) matrix: [[f32; 4]; 4],
    pub(crate) ortho_transform: u32
}

impl VertexAttrib for GeneralInstanceInput {
    const ATTRIBUTES: &[VertexAttribute] = &wgpu::vertex_attr_array![
        4 => Float32x3,
        5 => Float32,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
        9 => Float32x4,
        10 => Uint32,
    ];

    const STEP_MODE: VertexStepMode = wgpu::VertexStepMode::Instance;
}

#[repr(C)]
#[derive(Default, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShapeInstanceInput {
    pub(crate) position: [f32; 3],
    pub(crate) color_alpha: f32,
    pub(crate) matrix: [[f32; 4]; 4],
    pub(crate) bbox: [f32; 4],
    pub(crate) normal_scale: f32,
    pub _padding: [u32; 3],
}
impl VertexAttrib for ShapeInstanceInput {
    const ATTRIBUTES: &[VertexAttribute] = &wgpu::vertex_attr_array![
        5 => Float32x3,
        6 => Float32,
        7 => Float32x4,
        8 => Float32x4,
        9 => Float32x4,
        10 => Float32x4,
        11 => Float32x4,
        12 => Float32,
    ];

    const STEP_MODE: VertexStepMode = wgpu::VertexStepMode::Instance;
}

#[repr(C)]
#[derive(Default, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ScreenShapeInstanceInput {
    pub(crate) position: [f32; 3],
    pub(crate) color_alpha: f32,
    pub(crate) matrix: [[f32; 4]; 4],
    pub(crate) screen_space: u32,
}
impl VertexAttrib for ScreenShapeInstanceInput {
    const ATTRIBUTES: &[VertexAttribute] = &wgpu::vertex_attr_array![
        4 => Float32x3,
        5 => Float32,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
        9 => Float32x4,
        13 => Uint32,
    ];

    const STEP_MODE: VertexStepMode = wgpu::VertexStepMode::Instance;
}

