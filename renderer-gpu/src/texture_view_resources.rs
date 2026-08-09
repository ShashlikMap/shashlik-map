use crate::render_config::RenderConfig;
use crate::textures::create_depth_texture;
use wgpu::{Device, TextureFormat, TextureView};

pub(crate) struct TextureViewResources {
    pub texture_view_g_buf_positions: Option<TextureView>,
    pub texture_view_g_buf_normals: Option<TextureView>,
    pub texture_view_g_buf_depth: Option<TextureView>,
    pub texture_view_shadow_map_depth: TextureView,
    pub texture_view_ssao: Option<TextureView>,
}

impl TextureViewResources {
    pub(crate) fn new(render_config: &RenderConfig, device: &Device) -> Self {
        // TODO Currently, mesh pipeline expects it to be created from beginning.
        //  It's going be handled later during layers/pipeline redesign.
        let shadow_map_depth_texture = create_depth_texture(
            render_config.shadow_texture_size(),
            1,
            TextureFormat::Depth32Float,
            device,
        );
        Self {
            texture_view_g_buf_positions: None,
            texture_view_g_buf_normals: None,
            texture_view_g_buf_depth: None,
            texture_view_shadow_map_depth: shadow_map_depth_texture,
            texture_view_ssao: None,
        }
    }
}
