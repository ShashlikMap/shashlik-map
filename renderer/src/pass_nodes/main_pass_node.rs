use crate::global_context::GlobalContext;
use crate::mesh_layers::layers::Layers;
use crate::mesh_layers::BaseMeshLayer;
use crate::pass_nodes::PassNode;
use crate::textures::{create_common_texture, create_depth_texture, create_simple_texture, create_simple_texture_with_data, TextureData, SAMPLE_COUNT};
use std::borrow::Cow;
use glam::{Vec3, Vec4};
use wesl::include_wesl;
use wgpu::{BindGroup, CommandEncoder, ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, ImageSubresourceRange, ShaderModuleDescriptor, ShaderSource, StorageTextureAccess, TextureFormat, TextureUsages, TextureView, TextureViewDimension};
use rand::{random as other_random, rng, RngExt};
use rand::prelude::ThreadRng;
use wgpu_canvas::SSAO_ENABLED;

pub(crate) struct MainPassNode {
    msaa_texture_view: TextureView,
    pub non_msaa_texture_view_positions: TextureView,
    pub non_msaa_texture_view_normals: TextureView,
    depth_texture_view: TextureView,
    pub non_msaa_depth_texture_view: TextureView,
    ssao_bind_group: BindGroup,
    camera_ssao_bind_group: BindGroup,
    ssao_compute_pipeline: ComputePipeline
}

impl MainPassNode {

    pub fn new(global_context: &GlobalContext) -> Self {
        let size = (
            global_context.config().width,
            global_context.config().height,
        );

        let non_msaa_size = (
            global_context.config().width,
            global_context.config().height,
        );

        let device = global_context.device();

        let non_msaa_texture_view_positions = create_simple_texture(
            TextureData {
                sample_count: 1,
                size: non_msaa_size,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                format: TextureFormat::Rgba16Float,
            },
            global_context.device(),
        );
        let non_msaa_texture_view_normals = create_simple_texture(
            TextureData {
                sample_count: 1,
                size: non_msaa_size,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                format: TextureFormat::Rgba16Float,
            },
            global_context.device(),
        );

        let noise_texture = create_simple_texture_with_data( TextureData {
            sample_count: 1,
            size: (64, 64),
            usage: TextureUsages::TEXTURE_BINDING,
            format: TextureFormat::Rgba32Float,
        }, global_context.queue(), global_context.device(), bytemuck::cast_slice(&Self::generate_noise_texture_data()));

        let kernel_texture = create_simple_texture_with_data( TextureData {
            sample_count: 1,
            size: (16, 1),
            usage: TextureUsages::TEXTURE_BINDING,
            format: TextureFormat::Rgba32Float,
        }, global_context.queue(), global_context.device(), bytemuck::cast_slice(&Self::generate_noise_texture_qq_data()));

        let non_msaa_depth_texture_view = create_depth_texture(non_msaa_size, 1,
                                                               TextureFormat::Depth24Plus,
                                                               global_context.device());

        let ssao_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: StorageTextureAccess::WriteOnly,
                    format: TextureFormat::Rgba16Float,
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
            }, wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                },
                count: None,
            }, wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                },
                count: None,
            }, wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                },
                count: None,
            }, wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                },
                count: None,
            }],
            label: Some("ssao_bind_group_layout"),
        });

        let camera_ssao_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_ssao_pipeline_group_layout"),
        });

        let camera_ssao_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_ssao_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: global_context
                    .view_projection
                    .uniform_buffer
                    .as_entire_binding(),
            }],
            label: Some("ssao_camera_pipeline_bind_group"),
        });

        let ssao_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &ssao_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&global_context.ssao_texture),
            },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&non_msaa_texture_view_normals),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&non_msaa_texture_view_positions),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&noise_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&kernel_texture),
                }],
            label: Some("ssao_compute_bind_group"),
        });

        let ssao_pipeline_layout = global_context.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SSAO Pipeline Layout"),
            bind_group_layouts: &[&ssao_bind_group_layout, &camera_ssao_bind_group_layout],
            ..Default::default()
        });
        
        let ssao_shader = global_context.device().create_shader_module(ShaderModuleDescriptor {
            label: Some("ssao"),
            source: ShaderSource::Wgsl(Cow::from(include_wesl!("ssao"))),
        });

        let ssao_compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("ssao_compute_pipeline"),
            layout: Some(&ssao_pipeline_layout),
            module: &ssao_shader,
            entry_point: None,
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            msaa_texture_view: create_common_texture(size, SAMPLE_COUNT, global_context),
            non_msaa_texture_view_positions,
            non_msaa_texture_view_normals,
            depth_texture_view: create_depth_texture(size,
                                                     SAMPLE_COUNT,
                                                     TextureFormat::Depth24Plus,
                                                     global_context.device()),
            non_msaa_depth_texture_view,
            ssao_bind_group,
            camera_ssao_bind_group,
            ssao_compute_pipeline,
        }
    }

    fn generate_noise_texture_data() -> [[Vec4; 3]; 4096] {
        use core::array::from_fn;
        let mut rng = rng();
        from_fn(|_| [Self::generate_rnd_vec4(&mut rng),
            Self::generate_rnd_vec4(&mut rng),
            Self::generate_rnd_vec4(&mut rng)])
    }

    fn generate_rnd_vec4(rng: &mut ThreadRng) -> Vec4 {
        Vec4::new(rng.random_range(-1.0..1.0),
                  rng.random_range(-1.0..1.0),
                  rng.random_range(-1.0..1.0), 0.0)
    }

    fn generate_noise_texture_qq_data() -> [[Vec4; 3]; 16] {
        use core::array::from_fn;
        let mut rng = rng();
        from_fn(|_| [Self::generate_rnd_vec4_qq(&mut rng),
            Self::generate_rnd_vec4_qq(&mut rng),
            Self::generate_rnd_vec4_qq(&mut rng)])
    }

    fn generate_rnd_vec4_qq(rng: &mut ThreadRng) -> Vec4 {
        Vec4::new(rng.random_range(-1.0..1.0),
                  rng.random_range(-1.0..1.0),
                  rng.random_range(0.0..1.0), 0.0)
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

        let depth_attachment = wgpu::RenderPassDepthStencilAttachment {
            view: &self.depth_texture_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        };

        if unsafe { SSAO_ENABLED } {
            {
                let non_msaa_color_attachment_positions = wgpu::RenderPassColorAttachment {
                    view: &self.non_msaa_texture_view_positions,
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
                    view: &self.non_msaa_texture_view_normals,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0, // We use 1.0 here to simulate z normal for ground
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                };

                let non_msaa_depth_attachment = wgpu::RenderPassDepthStencilAttachment {
                    view: &self.non_msaa_depth_texture_view,
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

                global_context.is_g_buffer_render = true;
                global_context.is_preview_render = false;
                global_context.is_shadow_render = false;
                layers.render(&mut render_pass, global_context);
            }

            let ssao_texture = global_context.ssao_texture.texture();
            encoder.clear_texture(ssao_texture, &ImageSubresourceRange::default());
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("SSAO Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.ssao_compute_pipeline);
            compute_pass.set_bind_group(0, &self.ssao_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.camera_ssao_bind_group, &[]);
            let wg_x = (ssao_texture.size().width as f32 / 8.0).ceil() as u32;
            let wg_y = (ssao_texture.size().height as f32 / 8.0).ceil() as u32;
            compute_pass.dispatch_workgroups(wg_x, wg_y, 1);
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
            global_context.is_shadow_render = false;
            layers.render(&mut render_pass, global_context);
        }
    }
}
