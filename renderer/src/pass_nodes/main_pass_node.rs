use crate::global_context::GlobalContext;
use crate::mesh_layers::BaseMeshLayer;
use crate::mesh_layers::layers::Layers;
use crate::pass_nodes::PassNode;
use crate::textures::{SAMPLE_COUNT, create_common_texture, create_depth_texture};
use wgpu::{CommandEncoder, TextureView};

pub(crate) struct MainPassNode {
    msaa_texture_view: TextureView,
    non_msaa_texture_view_color: TextureView,
    non_msaa_texture_view_normals: TextureView,
    depth_texture_view: TextureView,
    non_msaa_depth_texture_view: TextureView,
}

impl MainPassNode {
    pub fn new(global_context: &GlobalContext) -> Self {
        let size = (
            global_context.config().width,
            global_context.config().height,
        );
        Self {
            msaa_texture_view: create_common_texture(size, SAMPLE_COUNT, global_context),
            non_msaa_texture_view_color: create_common_texture(size, 1, global_context),
            non_msaa_texture_view_normals: create_common_texture(size, 1, global_context),
            depth_texture_view: create_depth_texture(size, SAMPLE_COUNT, global_context),
            non_msaa_depth_texture_view: create_depth_texture(size, 1, global_context),
        }
    }
}

impl PassNode for MainPassNode {
    fn compute(
        &mut self,
        _encoder: &mut CommandEncoder,
        _layers: &mut Layers,
        _global_context: &mut GlobalContext,
    ) {
        // no special computes
    }

    fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        output_view: &TextureView,
        layers: &mut Layers,
        global_context: &mut GlobalContext,
    ) {
        let msaa_color_attachment = wgpu::RenderPassColorAttachment {
            view: &self.msaa_texture_view,
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
        };
        let non_msaa_color_attachment_color = wgpu::RenderPassColorAttachment {
            view: &self.non_msaa_texture_view_color,
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
        };
        let non_msaa_color_attachment_normals = wgpu::RenderPassColorAttachment {
            view: &self.non_msaa_texture_view_normals,
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
        };

        let depth_attachment = wgpu::RenderPassDepthStencilAttachment {
            view: &self.depth_texture_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        };

        let non_msaa_depth_attachment = wgpu::RenderPassDepthStencilAttachment {
            view: &self.non_msaa_depth_texture_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        };

        {
            let descriptor = wgpu::RenderPassDescriptor {
                label: Some("MRT Render Pass"),
                color_attachments: &[
                    Some(non_msaa_color_attachment_color),
                    Some(non_msaa_color_attachment_normals),
                ],
                depth_stencil_attachment: Some(non_msaa_depth_attachment),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            };

            let mut render_pass = encoder.begin_render_pass(&descriptor);

            global_context.is_g_buffer_render = true;
            global_context.is_preview_render = false;
            layers.render(&mut render_pass, global_context);
        }

        {
            let descriptor = wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(msaa_color_attachment)],
                depth_stencil_attachment: Some(depth_attachment),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            };

            let mut render_pass = encoder.begin_render_pass(&descriptor);

            global_context.is_g_buffer_render = false;
            global_context.is_preview_render = false;
            layers.render(&mut render_pass, global_context);
        }
    }
}
