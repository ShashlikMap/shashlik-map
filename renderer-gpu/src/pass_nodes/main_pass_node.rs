use crate::global_context::{GlobalContext};
use crate::mesh_layers::BaseMeshLayerNew;
use crate::mesh_layers::layers::Layers;
use crate::pass_nodes::{BACKGROUND_ATTACHMENT_COLOR, PassNode};
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::screen_mesh_pipeline::{ScreenMeshPipeline, TextureInfo};
use crate::pipelines::shape_pipeline::ShapePipeline;
use crate::textures::{SAMPLE_COUNT, create_common_texture, create_depth_texture};
use renderer_common::WorldShapeFeatureLayerTag;
use wgpu::{CommandEncoder, TextureFormat, TextureView};

pub(crate) struct MainPassNode {
    msaa_texture_view: TextureView,
    depth_texture_view: TextureView,
    default_mesh_pipeline: MeshPipeline,
    default_shape_pipeline: ShapePipeline,
    screen_shape_layer: ShapePipeline,
    feature_shape_pipelines: Vec<(String, ShapePipeline)>,
    preview_screen_mesh_pipeline: ScreenMeshPipeline,
    text_screen_mesh_pipeline: ScreenMeshPipeline,
    shadow_map_screen_mesh_pipeline: ScreenMeshPipeline,
    post_process_screen_mesh_pipeline: ScreenMeshPipeline,
}

impl MainPassNode {
    pub fn new(
        global_context: &GlobalContext,
        layers: &Layers,
        world_shape_feature_layer_tag: Vec<WorldShapeFeatureLayerTag>,
    ) -> Self {
        let size = (
            global_context.config().width,
            global_context.config().height,
        );

        let default_mesh_pipeline = MeshPipeline::new(global_context, true, true, true);
        let default_shape_pipeline = ShapePipeline::new(global_context, None, false, true);

        let screen_shape_layer =
            ShapePipeline::new(global_context, Some("vs_main_screen"), false, false);

        let mut preview_screen_mesh_pipeline = ScreenMeshPipeline::new(
            global_context,
            TextureInfo {
                use_texture: true,
                filterable: true,
                vs_shader: None,
                fs_shader: "fs_main_textured",
            },
            false,
        );
        preview_screen_mesh_pipeline.set_texture_view(
            layers.preview_mesh_layer.texture_view(),
            global_context.device(),
        );

        let mut shadow_map_screen_mesh_pipeline = ScreenMeshPipeline::new(
            global_context,
            TextureInfo {
                use_texture: true,
                filterable: false,
                vs_shader: Some("vs_main_sm"),
                fs_shader: "fs_main_sm",
            },
            true,
        );
        shadow_map_screen_mesh_pipeline.set_texture_view(
            layers.shadow_map_layer.texture_view(),
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

        let feature_shape_pipelines = world_shape_feature_layer_tag
            .iter()
            .map(|tag| {
                let pipeline = ShapePipeline::new(
                    global_context,
                    tag.vertex_shader,
                    tag.indirect,
                    tag.single_instance_step,
                );
                (tag.name.to_string(), pipeline)
            })
            .collect();

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
            default_shape_pipeline,
            screen_shape_layer,
            text_screen_mesh_pipeline,
            feature_shape_pipelines,
            preview_screen_mesh_pipeline,
            shadow_map_screen_mesh_pipeline,
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
        
        // TODO Can it go to pipeline?
        layers.shape_layer.disable_skip_mesh_feature = false;
        layers.shape_layer.render_new(
            &mut render_pass,
            &mut self.default_shape_pipeline,
            global_context,
        );

        layers.mesh_layer.render_new(
            &mut render_pass,
            &mut self.default_mesh_pipeline,
            global_context,
        );

        if global_context.is_shadow_mapping_enabled() {
            layers.shadow_map_layer.render_new(
                &mut render_pass,
                &mut self.shadow_map_screen_mesh_pipeline,
                global_context,
            );
        }
        if global_context.is_ssao_enabled() {
            layers.post_process_layer.render_new(
                &mut render_pass,
                &mut self.post_process_screen_mesh_pipeline,
                global_context,
            );
        }
        layers.screen_shape_layer.render_new(
            &mut render_pass,
            &mut self.screen_shape_layer,
            global_context,
        );

        layers.text_feature_layers.with_layer(|layer| {
            layer.render_new(
                &mut render_pass,
                &mut self.text_screen_mesh_pipeline,
                global_context,
            )
        });

        self.feature_shape_pipelines
            .iter_mut()
            .for_each(|(feature_tag, shape_pipeline)| {
                if let Some(layer) = layers.feature_layers.get_layer(feature_tag) {
                    layer.render_new(&mut render_pass, shape_pipeline, global_context)
                }
            });

        if global_context.preview_type().is_enabled() {
            layers.preview_mesh_layer.render_new(
                &mut render_pass,
                &mut self.preview_screen_mesh_pipeline,
                global_context,
            );
        }
    }
}
