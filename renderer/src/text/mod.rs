pub(crate) mod text_renderer;
pub mod glyph_tesselator;
mod default_face_wrapper;

use wgpu::{DepthStencilState, Device, SurfaceConfiguration};
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::{BrushBuilder, TextBrush};

pub fn create_default_text_brush<'a>(
    device: &Device,
    config: &SurfaceConfiguration,
    depth_state: DepthStencilState,
    multi_sample_state: wgpu::MultisampleState,
) -> TextBrush<FontRef<'a>> {
    let mut depth_state = depth_state.clone();
    depth_state.depth_write_enabled = false;
    BrushBuilder::using_font_bytes(include_bytes!("../font.ttf"))
        .unwrap()
        .with_depth_stencil(Some(depth_state))
        .with_multisample(multi_sample_state)
        .build(device, config.width, config.height, config.format)
}
