use glam::{DMat4, DVec3};
use strum::{Display, EnumIter, EnumString};
use wgpu::Texture;

pub mod wgpu_canvas;

pub struct RendererUpdateData {
    pub view_matrix: DMat4,
    pub view_light_matrix: DMat4,
    pub proj_matrix: DMat4,
    pub view_proj_matrix: DMat4,
    pub cs_offset: DVec3,
    pub scale: f32,
    pub eye_direction: DVec3,
    pub up: DVec3,
    pub scale_2d_3d: f32
}

pub trait Renderer {
    fn resize(&mut self, width: u32, height: u32);
    fn update(&mut self, data: RendererUpdateData);
    fn render(&mut self) -> Option<Texture>;
}

// TODO Proper config manager

#[derive(Eq, PartialEq, Copy, Clone, Hash, Display, EnumIter, EnumString)]
pub enum PreviewType {
    None, Camera, SSAO, SSAOPositions, SSAONormals, SSAODepth, ShadowMap
}

impl PreviewType {
    pub fn is_enabled(self) -> bool {
        self != PreviewType::None
    }
}

pub static mut SHADOWS_ENABLED: bool = true;
pub static mut SHADOWS_TEX_SIZE: (u32, u32) = (2048, 2048);
pub static mut SSAO_ENABLED: bool = true;
pub static mut PREVIEW_TYPE: PreviewType = PreviewType::None;

