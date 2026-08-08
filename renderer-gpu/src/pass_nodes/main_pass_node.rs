use crate::global_context::{GlobalContext, GlobalRenderStep};
use crate::mesh_layers::BaseMeshLayer;
use crate::mesh_layers::layers::Layers;
use crate::pass_nodes::{BACKGROUND_ATTACHMENT_COLOR, PassNode};
use crate::textures::{SAMPLE_COUNT, create_common_texture, create_depth_texture};
use wgpu::{CommandEncoder, TextureFormat, TextureView};

pub(crate) struct MainPassNode {
    msaa_texture_view: TextureView,
    depth_texture_view: TextureView,
}

impl MainPassNode {
    pub fn new(global_context: &GlobalContext) -> Self {
        let size = (
            global_context.config().width,
            global_context.config().height,
        );

        Self {
            msaa_texture_view: create_common_texture(size, SAMPLE_COUNT, global_context),
            depth_texture_view: create_depth_texture(
                size,
                SAMPLE_COUNT,
                TextureFormat::Depth24PlusStencil8,
                global_context.device(),
            ),
        }
    }
}

impl PassNode for MainPassNode {
    fn run(
        &self,
        encoder: &mut CommandEncoder,
        layers: &mut Layers,
        global_context: &mut GlobalContext,
    ) {
        let output_view = global_context.canvas.create_texture_view();
        let msaa_color_attachment = wgpu::RenderPassColorAttachment {
            view: &self.msaa_texture_view,
            resolve_target: Some(&output_view),
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(BACKGROUND_ATTACHMENT_COLOR),
                // FYI!! Discard output! It improves MSAA drastically on low-end devices
                store: wgpu::StoreOp::Discard,
            },
            depth_slice: None,
        };

        let depth_attachment = wgpu::RenderPassDepthStencilAttachment {
            view: &self.depth_texture_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(0),
                store: wgpu::StoreOp::Store,
            }),
        };

        let descriptor = wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(msaa_color_attachment)],
            depth_stencil_attachment: Some(depth_attachment),
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        };

        let mut render_pass = encoder.begin_render_pass(&descriptor);

        global_context.render_step = GlobalRenderStep::MainStep;
        layers.render(&mut render_pass, global_context);
    }
}
