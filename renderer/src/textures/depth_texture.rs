use crate::textures::msaa_texture::MultisampledTexture;
use crate::global_context::GlobalContext;
use wgpu::TextureView;

pub struct DepthTexture {
    pub view: TextureView,
}

impl DepthTexture {
    pub fn new(global_context: &GlobalContext, is_rt: bool) -> Self {
        pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
        let device = global_context.device();
        let config = global_context.config();

        let mut texture_extent = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        if is_rt {
            texture_extent = wgpu::Extent3d {
                width: config.width / 4,
                height: config.height / 4,
                depth_or_array_layers: 1,
            };
        }

        let texture_descriptor = &wgpu::TextureDescriptor {
            size: texture_extent,
            mip_level_count: 1,
            sample_count: MultisampledTexture::SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[DEPTH_FORMAT],
        };
        let texture = device.create_texture(texture_descriptor);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { view }
    }
}
