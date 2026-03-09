use crate::global_context::GlobalContext;
use crate::mesh_layers::layers::Layers;
use crate::mesh_layers::BaseMeshLayer;
use crate::pass_nodes::PassNode;
use crate::textures::{create_color_binding_texture, create_depth_texture, create_msaa_texture};
use wgpu::{CommandEncoder, TextureView};

pub(crate) struct RenderToTexturePassNode {
    msaa_texture_view: TextureView,
    msaa_texture_view2: TextureView,
    depth_texture_view: TextureView,
    pub rt_texture_view: TextureView,
}

impl RenderToTexturePassNode {
    pub fn new(global_context: &GlobalContext) -> Self {
        let size = (
            global_context.config().width / 4,
            global_context.config().height / 4,
        );
        Self {
            msaa_texture_view: create_msaa_texture(size, global_context),
            msaa_texture_view2: create_msaa_texture(size, global_context),
            depth_texture_view: create_depth_texture(size, global_context),
            rt_texture_view: create_color_binding_texture(size, global_context),
        }
    }
}

impl PassNode for RenderToTexturePassNode {
    fn compute(&mut self, _encoder: &mut CommandEncoder, _layers: &mut Layers, _global_context: &mut GlobalContext) {
        // no special computes
    }
    
    fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        _output_view: &TextureView,
        layers: &mut Layers,
        global_context: &mut GlobalContext,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.msaa_texture_view,
                resolve_target: Some(&self.rt_texture_view),
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
            }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_texture_view2,
                    resolve_target: None,
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
                view: &self.depth_texture_view,
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

        layers.is_preview = true;
        layers.render(&mut render_pass, global_context);
    }
}
