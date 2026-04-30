use strum::{Display, EnumIter, EnumString};

pub mod wgpu_canvas;

// TODO Proper config manager

#[derive(Eq, PartialEq, Copy, Clone, Hash, Display, EnumIter, EnumString)]
pub enum PreviewType {
    None, Camera, SSAO, SSAOPositions, SSAONormals, SSAODepth, ShadowMap
}

pub static mut SHADOWS_ENABLED: bool = true;
pub static mut SHADOWS_TEX_SIZE: (u32, u32) = (2048, 2048);
pub static mut SSAO_ENABLED: bool = true;
pub static mut PREVIEW_ENABLED: bool = true;
pub static mut PREVIEW_TYPE: PreviewType = PreviewType::None;
