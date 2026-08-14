use crate::global_context::GlobalContext;
use crate::mesh_layers::layers::Layers;
use crate::mesh_layers::BaseMeshLayerNew;
use crate::pass_nodes::PassNode;
use crate::pipelines::shadow_map_pipeline::ShadowMapPipeline;
use crate::texture_view_resources::TextureViewKind;
use wgpu::CommandEncoder;

pub(crate) struct ShadowPrepass {
    shadow_map_pipeline: ShadowMapPipeline,
}

impl ShadowPrepass {
    pub fn new(global_context: &GlobalContext) -> ShadowPrepass {
        ShadowPrepass {
            shadow_map_pipeline: ShadowMapPipeline::new(global_context),
        }
    }
}

impl PassNode for ShadowPrepass {

    fn run(
        &mut self,
        encoder: &mut CommandEncoder,
        layers: &mut Layers,
        global_context: &mut GlobalContext,
    ) {
        let depth_attachment = wgpu::RenderPassDepthStencilAttachment {
            view: global_context.texture_view_resources.get_or_unwrap(TextureViewKind::ShadowMapDepth),
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
        layers.mesh_layer.render_new(&mut render_pass, &mut self.shadow_map_pipeline, global_context)
    }
}
