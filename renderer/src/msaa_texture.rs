use crate::GlobalContext;
use wgpu::TextureView;

pub struct MultisampledTexture {
    pub view: TextureView,
}

impl MultisampledTexture {
    pub const SAMPLE_COUNT: u32 = 4;
    pub fn new(global_context: &GlobalContext) -> Self {
        let device = global_context.device();
        let config = global_context.config();
        
        let multisampled_texture_extent = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };

        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: multisampled_texture_extent,
            mip_level_count: 1,
            sample_count: Self::SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };
        let texture = device.create_texture(multisampled_frame_descriptor);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { view }
    }
}
