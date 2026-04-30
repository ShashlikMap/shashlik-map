pub mod wgpu_canvas;

// TODO Proper config manager

pub static mut SHADOWS_ENABLED: bool = true;
pub static mut SHADOWS_TEX_SIZE: (u32, u32) = (2048, 2048);
pub static mut SSAO_ENABLED: bool = true;
pub static mut PREVIEW_ENABLED: bool = true;
