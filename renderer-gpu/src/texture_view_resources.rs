use wgpu::TextureView;

// TODO Draft to support separate passes for gbuf and ssao
#[derive(Default)]
pub(crate) struct TextureViewResources {
    pub non_msaa_texture_view_positions: Option<TextureView>,
    pub non_msaa_texture_view_normals: Option<TextureView>,
    pub non_msaa_depth_texture_view: Option<TextureView>,
}
