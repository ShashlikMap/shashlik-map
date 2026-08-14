use crate::global_context::{GlobalContext, GlobalRenderStep};
use crate::mesh_layers::layers::Layers;
use crate::mesh_layers::BaseMeshLayer;
use crate::pass_nodes::{PassNode, BACKGROUND_ATTACHMENT_COLOR};
use crate::textures::{create_color_binding_texture, create_common_texture, create_depth_texture, SAMPLE_COUNT};
use wgpu::{CommandEncoder, TextureFormat, TextureView};

pub(crate) struct RenderToTexturePassNode {
    msaa_texture_view: TextureView,
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
            msaa_texture_view: create_common_texture(size, SAMPLE_COUNT, global_context),
            depth_texture_view: create_depth_texture(size, SAMPLE_COUNT,
                                                     TextureFormat::Depth24PlusStencil8,
                                                     global_context.device()),
            rt_texture_view: create_color_binding_texture(size, global_context),
        }
    }
}

impl PassNode for RenderToTexturePassNode {
 
    fn run(
        &self,
        encoder: &mut CommandEncoder,
        layers: &mut Layers,
        global_context: &mut GlobalContext,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render To Texture Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.msaa_texture_view,
                resolve_target: Some(&self.rt_texture_view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(BACKGROUND_ATTACHMENT_COLOR),
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

        global_context.render_step = GlobalRenderStep::PreviewStep;
        layers.shape_layer.disable_skip_mesh_feature = true;
        layers.shape_layer.render(&mut render_pass, global_context);
        layers.feature_layers.render(&mut render_pass, global_context);
    }
}
