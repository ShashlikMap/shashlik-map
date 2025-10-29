use wgpu::{Device, TextureView};
use crate::msaa_texture::MultisampledTexture;

pub struct DepthTexture {
    pub view: TextureView,
}

impl DepthTexture {
    pub fn new(device: &Device, width: u32, height: u32) -> Self {
        pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

        let multisampled_texture_extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: multisampled_texture_extent,
            mip_level_count: 1,
            sample_count: MultisampledTexture::SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[DEPTH_FORMAT],
        };
        let texture = device.create_texture(multisampled_frame_descriptor);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { view }
    }
}
