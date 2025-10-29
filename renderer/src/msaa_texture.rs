use wgpu::{Device, TextureView};

pub struct MultisampledTexture {
    pub view: TextureView,
}

impl MultisampledTexture {
    pub const SAMPLE_COUNT: u32 = 4;
    pub fn new(device: &Device, width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
        let multisampled_texture_extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: multisampled_texture_extent,
            mip_level_count: 1,
            sample_count: Self::SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };
        let texture = device.create_texture(multisampled_frame_descriptor);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { view }
    }
}
