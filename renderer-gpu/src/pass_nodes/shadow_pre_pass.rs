use crate::global_context::{GlobalContext, GlobalRenderStep};
use crate::mesh_layers::layers::Layers;
use crate::mesh_layers::BaseMeshLayer;
use crate::pass_nodes::PassNode;
use wgpu::{CommandEncoder, TextureView};

pub(crate) struct ShadowPrepass {}

impl ShadowPrepass {
    pub fn new() -> ShadowPrepass {
        ShadowPrepass {}
    }
}

impl PassNode for ShadowPrepass {
    fn compute(
        &self,
        _encoder: &mut CommandEncoder,
        _layers: &mut Layers,
        _global_context: &mut GlobalContext,
    ) {
    }

    fn render(
        &self,
        encoder: &mut CommandEncoder,
        _output_view: &TextureView,
        layers: &mut Layers,
        global_context: &mut GlobalContext,
    ) {
        if !global_context.is_shadow_mapping_enabled() {
            return;
        }
        let depth_attachment = wgpu::RenderPassDepthStencilAttachment {
            view: &global_context.shadow_map_depth_texture,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        };

        let descriptor = wgpu::RenderPassDescriptor {
            label: Some("Shadow Render Pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(depth_attachment),
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        };

        let mut render_pass = encoder.begin_render_pass(&descriptor);

        global_context.render_step = GlobalRenderStep::ShadowStep;

        layers.render(&mut render_pass, global_context);
    }
}
