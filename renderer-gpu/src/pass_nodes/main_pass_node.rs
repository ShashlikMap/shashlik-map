use std::borrow::Cow;
use std::fs;
use crate::DEPTH_STENCIL_TEX_FORMAT;
use crate::global_context::GlobalContext;
use crate::mesh_layers::RenderableLayer;
use crate::mesh_layers::layers::Layers;
use crate::pass_nodes::{PassNode, BACKGROUND_ATTACHMENT_COLOR, GLOBE_BACKGROUND_ATTACHMENT_COLOR};
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::screen_mesh_pipeline::{ScreenMeshPipeline, TextureInfo};
use crate::pipelines::shape_pipeline::ShapePipeline;
use crate::pipelines::x_real_mesh_pipeline::XRealMeshShaderPipeline;
use crate::textures::{SAMPLE_COUNT, create_common_texture, create_depth_texture};
use renderer_common::WorldShapeFeatureLayerTag;
use wgpu::{AstcBlock, AstcChannel, CommandEncoder, Extent3d, TextureView};
use wgpu::Face::Back;
use wgpu::util::DeviceExt;
use crate::buffer_pool::BufferPool;

pub(crate) struct MainPassNode {
    msaa_texture_view: TextureView,
    depth_texture_view: TextureView,
    default_mesh_pipeline: MeshPipeline,
    x_real_mesh_shader_pipeline: XRealMeshShaderPipeline,
    default_shape_pipeline: ShapePipeline,
    screen_shape_pipeline: ShapePipeline,
    feature_shape_pipelines: Vec<(String, ShapePipeline)>,
    preview_screen_mesh_pipeline: Option<ScreenMeshPipeline>,
    text_screen_mesh_pipeline: ScreenMeshPipeline,
    post_process_screen_mesh_pipeline: Option<ScreenMeshPipeline>,
    stars_pipeline: ScreenMeshPipeline,
    globe_pipeline: ScreenMeshPipeline,
    globe_glow_pipeline: ScreenMeshPipeline,
}

const STARS_BYTES: &'static [u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../stars-cubemap/astc.ktx2"));

impl MainPassNode {
    pub fn new(
        global_context: &GlobalContext,
        x_real_mesh_shader_pipeline_enabled: bool,
        layers: &mut Layers,
        world_shape_feature_layer_tag: Vec<WorldShapeFeatureLayerTag>,
        buffer_pool: &mut BufferPool
    ) -> Self {
        let size = (
            global_context.config().width,
            global_context.config().height,
        );

        let default_mesh_pipeline = MeshPipeline::new(global_context, true, false, true);

        let x_real_mesh_shader_pipeline = XRealMeshShaderPipeline::new(global_context,
                                                                       x_real_mesh_shader_pipeline_enabled);

        let default_shape_pipeline = ShapePipeline::new(global_context, None, false, true, Some(Back));

        let screen_shape_pipeline =
            ShapePipeline::new(global_context, Some("vs_main_screen"), false, false, None);

        let tv = layers.preview_mesh_layer.texture_view();
        let preview_screen_mesh_pipeline = tv.map(|_| {
            ScreenMeshPipeline::new(
                global_context,
                TextureInfo {
                    use_texture: tv.is_some(),
                    filterable: false, // ideally, it should be picked using underlying TextureFormat..
                    fs_shader: "fs_main_textured",
                },
                tv,
                false,
            )
        });

        let tv = layers.post_process_layer.texture_view();
        let post_process_screen_mesh_pipeline = tv.map(|_| {
            ScreenMeshPipeline::new(
                global_context,
                TextureInfo {
                    use_texture: tv.is_some(),
                    filterable: true,
                    fs_shader: "fs_main_tex_storage",
                },
                tv,
                false,
            )
        });

        let feature_shape_pipelines = ShapePipeline::from_world_shape_tags(global_context,
                                                                           world_shape_feature_layer_tag);

        let text_screen_mesh_pipeline = ScreenMeshPipeline::new(
            global_context,
            TextureInfo {
                use_texture: false,
                filterable: false,
                fs_shader: "",
            },
            None,
            false,
        );

        let globe_pipeline = ScreenMeshPipeline::new(
            global_context,
            TextureInfo {
                use_texture: false,
                filterable: false,
                fs_shader: "fs_main_globe",
            },
            None,
            false,
        );
        layers.globe_layer.set_texture(None, (0.0, 0.0), global_context, buffer_pool);

        let globe_glow_pipeline = ScreenMeshPipeline::new(
            global_context,
            TextureInfo {
                use_texture: false,
                filterable: false,
                fs_shader: "fs_main_globe_glow",
            },
            None,
            false,
        );

        let star_box_format = if global_context.device().features().contains(wgpu::Features::TEXTURE_COMPRESSION_ASTC) {
            wgpu::TextureFormat::Astc {
                block: AstcBlock::B4x4,
                channel: AstcChannel::UnormSrgb,
            }
        } else {
            wgpu::TextureFormat::Bc7RgbaUnormSrgb
        };

        let stars_bytes = if star_box_format == wgpu::TextureFormat::Bc7RgbaUnormSrgb {
            Cow::Owned(fs::read("/stars-cubemap/bc7.ktx2").unwrap())
        } else {
            Cow::Borrowed(STARS_BYTES)
        };

        let reader = ktx2::Reader::new(stars_bytes).unwrap();
        let header = reader.header();
        let mut image = Vec::with_capacity(reader.data().len());
        for level in reader.levels() {
            image.extend_from_slice(level.data);
        }

        let texture = global_context.device().create_texture_with_data(
            global_context.queue(),
            &wgpu::TextureDescriptor {
                size: Extent3d {
                    width: 512,
                    height: 512,
                    depth_or_array_layers: 6,
                },
                mip_level_count: header.level_count,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: star_box_format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: None,
                view_formats: &[],
            },
            // KTX2 stores mip levels in mip major order.
            wgpu::util::TextureDataOrder::MipMajor,
            &image,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..wgpu::TextureViewDescriptor::default()
        });

        let stars_pipeline = ScreenMeshPipeline::new(
            global_context,
            TextureInfo {
                use_texture: true,
                filterable: true,
                fs_shader: "fs_main_stars",
            },
            Some(&texture_view),
            false,
        );

        Self {
            msaa_texture_view: create_common_texture(size, SAMPLE_COUNT, global_context),
            depth_texture_view: create_depth_texture(
                size,
                SAMPLE_COUNT,
                DEPTH_STENCIL_TEX_FORMAT,
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
            stars_pipeline,
            globe_pipeline,
            globe_glow_pipeline
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
        let clear_color = if global_context.is_globe_view() {
            GLOBE_BACKGROUND_ATTACHMENT_COLOR
        } else {
            BACKGROUND_ATTACHMENT_COLOR
        };
        let output_view = global_context.canvas.create_texture_view();
        let msaa_color_attachment = wgpu::RenderPassColorAttachment {
            view: &self.msaa_texture_view,
            resolve_target: Some(&output_view),
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear_color),
                // FYI!! Discard output! It improves MSAA drastically on low-end devices
                store: wgpu::StoreOp::Discard,
            },
            depth_slice: None,
        };

        let depth_attachment = wgpu::RenderPassDepthStencilAttachment {
            view: &self.depth_texture_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Discard,
            }),
            stencil_ops: DEPTH_STENCIL_TEX_FORMAT.has_stencil_aspect().then_some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(0),
                store: wgpu::StoreOp::Discard,
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

        if global_context.is_globe_view() {
            layers.globe_layer.render(&mut render_pass,
                                      &mut self.stars_pipeline,
                                      global_context);
            layers.globe_layer.render(&mut render_pass,
                                      &mut self.globe_pipeline,
                                      global_context);
        }

        layers.shape_layer.disable_skip_mesh_feature = false;
        layers.shape_layer.render(
            &mut render_pass,
            &mut self.default_shape_pipeline,
            global_context,
        );

        if global_context.is_globe_view() && global_context.is_globe_visible() {
            layers.globe_layer.render(&mut render_pass,
                                      &mut self.globe_glow_pipeline,
                                      global_context);
        }


        if global_context.x_real_mesh_shader_enabled {
            layers.mesh_layer.render(
                &mut render_pass,
                &mut self.x_real_mesh_shader_pipeline,
                global_context,
            );
        }

        // 3D rendering only if 3D visible, including transition to 2D
        // TODO Ideally, it should be possible to control outside of the renderer
        if global_context.view_projection.scale_2d_3d > 0.0 {
            layers.mesh_layer.render_with_virtual_ground(
                &mut render_pass,
                &mut self.default_mesh_pipeline,
                global_context,
                true
            );

            if global_context.is_ssao_enabled()
                && let Some(post_process_screen_mesh_pipeline) = self.post_process_screen_mesh_pipeline.as_mut() {
                layers.post_process_layer.render(
                    &mut render_pass,
                    post_process_screen_mesh_pipeline,
                    global_context,
                );
            }
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

        if global_context.preview_type().is_enabled()
            && let Some(preview_screen_mesh_pipeline) = self.preview_screen_mesh_pipeline.as_mut() {
            layers.preview_mesh_layer.render(
                &mut render_pass,
                preview_screen_mesh_pipeline,
                global_context,
            );
        }
    }
}
