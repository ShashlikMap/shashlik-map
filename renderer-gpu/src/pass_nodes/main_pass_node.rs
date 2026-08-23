use crate::global_context::{GlobalContext};
use crate::mesh_layers::RenderableLayer;
use crate::mesh_layers::layers::Layers;
use crate::pass_nodes::{BACKGROUND_ATTACHMENT_COLOR, PassNode};
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::screen_mesh_pipeline::{ScreenMeshPipeline, TextureInfo};
use crate::pipelines::shape_pipeline::ShapePipeline;
use crate::textures::{SAMPLE_COUNT, create_common_texture, create_depth_texture};
use renderer_common::WorldShapeFeatureLayerTag;
use wgpu::{CommandEncoder, TextureFormat, TextureView};
use crate::pipelines::x_real_mesh_pipeline::XRealMeshShaderPipeline;

pub(crate) struct MainPassNode {
    msaa_texture_view: TextureView,
    depth_texture_view: TextureView,
    default_mesh_pipeline: MeshPipeline,
    x_real_mesh_shader_pipeline: XRealMeshShaderPipeline,
    default_shape_pipeline: ShapePipeline,
    screen_shape_pipeline: ShapePipeline,
    feature_shape_pipelines: Vec<(String, ShapePipeline)>,
    preview_screen_mesh_pipeline: ScreenMeshPipeline,
    text_screen_mesh_pipeline: ScreenMeshPipeline,
    post_process_screen_mesh_pipeline: ScreenMeshPipeline,
}

impl MainPassNode {
    pub fn new(
        global_context: &GlobalContext,
        x_real_mesh_shader_pipeline_enabled: bool,
        layers: &Layers,
        world_shape_feature_layer_tag: Vec<WorldShapeFeatureLayerTag>,
    ) -> Self {
        let size = (
            global_context.config().width,
            global_context.config().height,
        );

        let default_mesh_pipeline = MeshPipeline::new(global_context, true, true, true);

        let x_real_mesh_shader_pipeline = XRealMeshShaderPipeline::new(global_context,
                                                                       x_real_mesh_shader_pipeline_enabled);

        let default_shape_pipeline = ShapePipeline::new(global_context, None, false, true);

        let screen_shape_pipeline =
            ShapePipeline::new(global_context, Some("vs_main_screen"), false, false);

        let mut preview_screen_mesh_pipeline = ScreenMeshPipeline::new(
            global_context,
            TextureInfo {
                use_texture: true,
                filterable: false, // ideally, it should be picked using underlying TextureFormat..
                vs_shader: None,
                fs_shader: "fs_main_textured",
            },
            false,
        );
        preview_screen_mesh_pipeline.set_texture_view(
            layers.preview_mesh_layer.texture_view(),
            global_context.device(),
        );

        let mut post_process_screen_mesh_pipeline = ScreenMeshPipeline::new(
            global_context,
            TextureInfo {
                use_texture: true,
                filterable: true,
                vs_shader: None,
                fs_shader: "fs_main_tex_storage",
            },
            false,
        );
        post_process_screen_mesh_pipeline.set_texture_view(
            layers.post_process_layer.texture_view(),
            global_context.device(),
        );

        let feature_shape_pipelines = ShapePipeline::from_world_shape_tags(global_context,
                                                                           world_shape_feature_layer_tag);

        let text_screen_mesh_pipeline = ScreenMeshPipeline::new(
            global_context,
            TextureInfo {
                use_texture: false,
                filterable: false,
                vs_shader: None,
                fs_shader: "",
            },
            false,
        );

        Self {
            msaa_texture_view: create_common_texture(size, SAMPLE_COUNT, global_context),
            depth_texture_view: create_depth_texture(
                size,
                SAMPLE_COUNT,
                TextureFormat::Depth24PlusStencil8,
                global_context.device(),
            ),
            default_mesh_pipeline,
            x_real_mesh_shader_pipeline,
            default_shape_pipeline,
            screen_shape_pipeline,
            text_screen_mesh_pipeline,
            feature_shape_pipelines,
            preview_screen_mesh_pipeline,
            post_process_screen_mesh_pipeline,
        }
    }
}

impl PassNode for MainPassNode {
    fn run(
        &mut self,
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
        
        layers.shape_layer.disable_skip_mesh_feature = false;
        layers.shape_layer.render(
            &mut render_pass,
            &mut self.default_shape_pipeline,
            global_context,
        );

        if global_context.x_real_mesh_shader_enabled {
            layers.mesh_layer.render(
                &mut render_pass,
                &mut self.x_real_mesh_shader_pipeline,
                global_context,
            );
        }

        layers.mesh_layer.render(
            &mut render_pass,
            &mut self.default_mesh_pipeline,
            global_context,
        );
        
        if global_context.is_ssao_enabled() {
            layers.post_process_layer.render(
                &mut render_pass,
                &mut self.post_process_screen_mesh_pipeline,
                global_context,
            );
        }
        layers.screen_shape_layer.render(
            &mut render_pass,
            &mut self.screen_shape_pipeline,
            global_context,
        );

        layers.text_feature_layers.with_layer(|layer| {
            layer.render(
                &mut render_pass,
                &mut self.text_screen_mesh_pipeline,
                global_context,
            )
        });

        self.feature_shape_pipelines
            .iter_mut()
            .for_each(|(feature_tag, shape_pipeline)| {
                if let Some(layer) = layers.feature_layers.get_layer(feature_tag) {
                    layer.render(&mut render_pass, shape_pipeline, global_context)
                }
            });

        if global_context.preview_type().is_enabled() {
            layers.preview_mesh_layer.render(
                &mut render_pass,
                &mut self.preview_screen_mesh_pipeline,
                global_context,
            );
        }
    }
}
