use crate::render_config::RenderConfig;
use crate::textures::create_depth_texture;
use wgpu::{Device, TextureFormat, TextureView};

// TODO Draft to support separate passes for gbuf and ssao
pub(crate) struct TextureViewResources {
    pub non_msaa_texture_view_positions: Option<TextureView>,
    pub non_msaa_texture_view_normals: Option<TextureView>,
    pub non_msaa_depth_texture_view: Option<TextureView>,
    pub shadow_map_depth_texture: TextureView,
    pub ssao_texture: Option<TextureView>,
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
            non_msaa_texture_view_positions: None,
            non_msaa_texture_view_normals: None,
            non_msaa_depth_texture_view: None,
            shadow_map_depth_texture,
            ssao_texture: None,
        }
    }
}
