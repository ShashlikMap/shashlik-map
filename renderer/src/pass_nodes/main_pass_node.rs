use crate::global_context::GlobalContext;
use crate::mesh_layers::BaseMeshLayer;
use crate::mesh_layers::layers::Layers;
use crate::pass_nodes::PassNode;
use crate::textures::depth_texture::DepthTexture;
use crate::textures::msaa_texture::MultisampledTexture;
use wgpu::{CommandEncoder, TextureView};

pub(crate) struct MainPassNode {
    msaa_texture: MultisampledTexture,
    depth_texture: DepthTexture,
}

impl MainPassNode {
    pub fn new(global_context: &GlobalContext) -> Self {
        Self {
            msaa_texture: MultisampledTexture::new(global_context, false),
            depth_texture: DepthTexture::new(global_context, false),
        }
    }
}

impl PassNode for MainPassNode {
    fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        output_view: &TextureView,
        layers: &mut Layers,
        global_context: &mut GlobalContext,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.msaa_texture.view,
                resolve_target: Some(output_view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.741,
                        b: 0.961,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });

        layers.is_preview = false;
        layers.render(&mut render_pass, global_context);
    }
}
