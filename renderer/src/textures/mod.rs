use crate::global_context::GlobalContext;
use wgpu::{Device, TextureFormat, TextureUsages, TextureView};
pub(crate) struct TextureData {
    pub(crate) sample_count: u32,
    pub(crate) size: (u32, u32),
    pub(crate) usage: TextureUsages,
    pub(crate) format: TextureFormat,
}

pub const SAMPLE_COUNT: u32 = 4;

pub fn create_color_binding_texture(
    size: (u32, u32),
    global_context: &GlobalContext,
) -> TextureView {
    let config = global_context.config();
    create_simple_texture(
        TextureData {
            sample_count: 1,
            size,
            usage: TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            format: config.format,
        },
        global_context.device(),
    )
}

pub fn create_common_texture(size: (u32, u32), sample_count: u32, global_context: &GlobalContext) -> TextureView {
    let config = global_context.config();
    create_simple_texture(
        TextureData {
            sample_count,
            size,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            format: config.format,
        },
        global_context.device(),
    )
}

pub fn create_depth_texture(size: (u32, u32), sample_count: u32, global_context: &GlobalContext) -> TextureView {
    create_simple_texture(
        TextureData {
            sample_count,
            size,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            format: TextureFormat::Depth32Float,
        },
        global_context.device(),
    )
}

pub fn create_simple_texture(texture_data: TextureData, device: &Device) -> TextureView {

    let texture_extent = wgpu::Extent3d {
        width: texture_data.size.0,
        height: texture_data.size.1,
        depth_or_array_layers: 1,
    };

    let texture_descriptor = &wgpu::TextureDescriptor {
        size: texture_extent,
        mip_level_count: 1,
        sample_count: texture_data.sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: texture_data.format,
        usage: texture_data.usage,
        label: None,
        view_formats: &[],
    };
    let texture = device.create_texture(texture_descriptor);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    view
}
