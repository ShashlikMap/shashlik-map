use crate::global_context::GlobalContext;
use wgpu::TextureView;

pub struct RtTexture {
    pub view: TextureView,
}

impl RtTexture {
    pub fn new(global_context: &GlobalContext) -> Self {
        let device = global_context.device();
        let config = global_context.config();

        let texture_extent = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };

        let frame_descriptor = &wgpu::TextureDescriptor {
            size: texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };
        let texture = device.create_texture(frame_descriptor);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { view }
    }
}
