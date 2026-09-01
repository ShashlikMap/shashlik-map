use crate::DEPTH_STENCIL_TEX_FORMAT;
use crate::global_context::GlobalContext;
use crate::mesh_layers::layers::Layers;
use crate::pass_nodes::{BACKGROUND_ATTACHMENT_COLOR, PassNode};
use crate::pipelines::shape_pipeline::ShapePipeline;
use crate::textures::{
    SAMPLE_COUNT, create_color_binding_texture, create_common_texture, create_depth_texture,
};
use renderer_common::WorldShapeFeatureLayerTag;
use wgpu::{CommandEncoder, TextureView};

pub(crate) struct RenderToTexturePassNode {
    msaa_texture_view: TextureView,
    depth_texture_view: TextureView,
    pub rt_texture_view: TextureView,
    shape_pipeline: ShapePipeline,
    feature_shape_pipelines: Vec<(String, ShapePipeline)>,
}

impl RenderToTexturePassNode {
    pub fn new(
        global_context: &GlobalContext,
        world_shape_feature_layer_tag: Vec<WorldShapeFeatureLayerTag>,
    ) -> Self {
        let size = (
            global_context.config().width / 4,
            global_context.config().height / 4,
        );
        let feature_shape_pipelines = ShapePipeline::from_world_shape_tags(global_context,
                                                                           world_shape_feature_layer_tag);
        Self {
            msaa_texture_view: create_common_texture(size, SAMPLE_COUNT, global_context),
            depth_texture_view: create_depth_texture(
                size,
                SAMPLE_COUNT,
                DEPTH_STENCIL_TEX_FORMAT,
                global_context.device(),
            ),
            rt_texture_view: create_color_binding_texture(size, global_context),
            shape_pipeline: ShapePipeline::new(global_context, None, false, true),
            feature_shape_pipelines,
        }
    }
}

impl PassNode for RenderToTexturePassNode {
    fn run(
        &mut self,
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

        layers.shape_layer.disable_skip_mesh_feature = true;
        layers
            .shape_layer
            .render(&mut render_pass, &mut self.shape_pipeline, global_context);
        self.feature_shape_pipelines
            .iter_mut()
            .for_each(|(feature_tag, shape_pipeline)| {
                if let Some(layer) = layers.feature_layers.get_layer(feature_tag) {
                    layer.render(&mut render_pass, shape_pipeline, global_context)
                }
            });
    }
}
