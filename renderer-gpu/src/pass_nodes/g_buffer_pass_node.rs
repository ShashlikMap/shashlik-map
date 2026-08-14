use crate::global_context::{GlobalContext, GlobalRenderStep};
use crate::mesh_layers::BaseMeshLayer;
use crate::mesh_layers::layers::Layers;
use crate::pass_nodes::PassNode;
use crate::textures::{TextureData, create_depth_texture, create_simple_texture};
use wgpu::{CommandEncoder, TextureFormat, TextureUsages};
use crate::texture_view_resources::TextureViewKind;

pub(crate) struct GBufferPassNode;

impl GBufferPassNode {
    pub fn new(global_context: &mut GlobalContext) -> Self {
        let non_msaa_size = (
            global_context.config().width,
            global_context.config().height,
        );
        let non_msaa_texture_view_positions = create_simple_texture(
            TextureData {
                sample_count: 1,
                size: non_msaa_size,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                format: TextureFormat::Rgba16Float,
            },
            global_context.device(),
        );
        global_context
            .texture_view_resources.insert(TextureViewKind::GBufPositions, non_msaa_texture_view_positions);

        let non_msaa_texture_view_normals = create_simple_texture(
            TextureData {
                sample_count: 1,
                size: non_msaa_size,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                format: TextureFormat::Rgba16Float,
            },
            global_context.device(),
        );
        global_context
            .texture_view_resources.insert(TextureViewKind::GBufNormals, non_msaa_texture_view_normals);

        let non_msaa_depth_texture_view = create_depth_texture(
            non_msaa_size,
            1,
            TextureFormat::Depth24Plus,
            global_context.device(),
        );

        global_context
            .texture_view_resources.insert(TextureViewKind::GBufDepth, non_msaa_depth_texture_view);

        Self
    }
}

impl PassNode for GBufferPassNode {
    fn run(
        &self,
        encoder: &mut CommandEncoder,
        layers: &mut Layers,
        global_context: &mut GlobalContext,
    ) {
        let non_msaa_color_attachment_positions = wgpu::RenderPassColorAttachment {
            view: global_context
                .texture_view_resources
                .get_or_unwrap(TextureViewKind::GBufPositions),
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        };
        let non_msaa_color_attachment_normals = wgpu::RenderPassColorAttachment {
            view: global_context
                .texture_view_resources
                .get_or_unwrap(TextureViewKind::GBufNormals),
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        };

        let non_msaa_depth_attachment = wgpu::RenderPassDepthStencilAttachment {
            view: global_context
                .texture_view_resources
                .get_or_unwrap(TextureViewKind::GBufDepth),
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        };

        let descriptor = wgpu::RenderPassDescriptor {
            label: Some("MRT Render Pass"),
            color_attachments: &[
                Some(non_msaa_color_attachment_positions),
                Some(non_msaa_color_attachment_normals),
            ],
            depth_stencil_attachment: Some(non_msaa_depth_attachment),
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        };

        let mut render_pass = encoder.begin_render_pass(&descriptor);

        global_context.render_step = GlobalRenderStep::GBufferStep;
        layers.mesh_layer.render(&mut render_pass, global_context);
    }
}
