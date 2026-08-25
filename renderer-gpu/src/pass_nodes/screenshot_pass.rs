use crate::global_context::GlobalContext;
use crate::mesh_layers::layers::Layers;
use crate::pass_nodes::PassNode;
use crate::utils::TextureExt;
use wgpu::CommandEncoder;

pub(crate) struct ScreenshotPass();

impl PassNode for ScreenshotPass {
    fn run(
        &mut self,
        encoder: &mut CommandEncoder,
        _layers: &mut Layers,
        global_context: &mut GlobalContext,
    ) {
        let output_texture = global_context.canvas.texture().unwrap();

        let output_buffer_size = (output_texture.padded_bytes_per_row() * output_texture.height())
            as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: None,
            mapped_at_creation: false,
        };
        let output_buffer = global_context.device().create_buffer(&output_buffer_desc);

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: output_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    bytes_per_row: Some(output_texture.padded_bytes_per_row()),
                    ..Default::default()
                },
            },
            output_texture.size(),
        );
        global_context.set_png_buffer(output_buffer);
    }
}
