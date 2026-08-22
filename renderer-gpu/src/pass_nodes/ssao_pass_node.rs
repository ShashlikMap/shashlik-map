use crate::global_context::GlobalContext;
use crate::mesh_layers::layers::Layers;
use crate::pass_nodes::PassNode;
use crate::texture_view_resources::TextureViewKind;
use crate::textures::{TextureData, create_simple_texture, create_simple_texture_with_data};
use glam::Vec4;
use rand::{RngExt, rng};
use std::borrow::Cow;
use wesl::include_wesl;
use wgpu::{
    BindGroup, CommandEncoder, ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor,
    ImageSubresourceRange, ShaderModuleDescriptor, ShaderSource, StorageTextureAccess,
    TextureFormat, TextureUsages, TextureViewDimension,
};

pub(crate) struct SsaoPassNode {
    ssao_bind_group: BindGroup,
    camera_ssao_bind_group: BindGroup,
    ssao_compute_pipeline: ComputePipeline,
}

impl SsaoPassNode {
    // TODO Sync with ones in shader, someday
    const NOISE_SIZE: usize = 16;

    const KERNEL_SIZE: usize = 16;

    pub fn new(global_context: &mut GlobalContext) -> Self {
        let device = global_context.device();
        let canvas = &global_context.canvas;

        // SSAO is already expensive, let's try to make it better with one texture size first
        // don't scale it down
        let ssao_size = (canvas.config().width, canvas.config().height);

        let ssao_texture = create_simple_texture(
            TextureData {
                sample_count: 1,
                size: ssao_size,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
                format: TextureFormat::Rgba16Float,
            },
            device,
        );

        const {
            assert!(Self::NOISE_SIZE.isqrt().pow(2) == Self::NOISE_SIZE);
        };
        let size = Self::NOISE_SIZE.isqrt() as u32;
        let noise_texture = create_simple_texture_with_data(
            TextureData {
                sample_count: 1,
                size: (size, size),
                usage: TextureUsages::TEXTURE_BINDING,
                format: TextureFormat::Rgba32Float,
            },
            global_context.queue(),
            global_context.device(),
            bytemuck::cast_slice(&Self::generate_noise_texture_data::<{Self::NOISE_SIZE}>()),
        );

        let kernel_texture = create_simple_texture_with_data(
            TextureData {
                sample_count: 1,
                size: (Self::KERNEL_SIZE as u32, 1),
                usage: TextureUsages::TEXTURE_BINDING,
                format: TextureFormat::Rgba32Float,
            },
            global_context.queue(),
            global_context.device(),
            bytemuck::cast_slice(&Self::generate_ssao_kernel_data::<{Self::KERNEL_SIZE}>()),
        );

        let camera_ssao_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let ssao_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rgba16Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                ],
                label: Some("ssao_bind_group_layout"),
            });

        let ssao_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &ssao_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&ssao_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        global_context
                            .texture_view_resources
                            .get_or_unwrap(TextureViewKind::GBufPositions),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        global_context
                            .texture_view_resources
                            .get_or_unwrap(TextureViewKind::GBufNormals),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&noise_texture),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&kernel_texture),
                },
            ],
            label: Some("ssao_compute_bind_group"),
        });

        let ssao_pipeline_layout =
            global_context
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("SSAO Pipeline Layout"),
                    bind_group_layouts: &[
                        Some(&camera_ssao_bind_group_layout),
                        Some(&ssao_bind_group_layout),
                    ],
                    ..Default::default()
                });

        let ssao_shader = global_context
            .device()
            .create_shader_module(ShaderModuleDescriptor {
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

        global_context.texture_view_resources.insert(TextureViewKind::SSAO, ssao_texture);

        Self {
            ssao_bind_group,
            camera_ssao_bind_group,
            ssao_compute_pipeline,
        }
    }

    fn generate_noise_texture_data<const N: usize>() -> [Vec4; N] {
        use core::array::from_fn;
        let mut rng = rng();
        from_fn(|i| {
            let angle = (i as f32 + rng.random_range(0.0..1.0)) / (N as f32) * std::f32::consts::TAU;
            Vec4::new(angle.cos(), angle.sin(), 0.0, 0.0)
        })
    }

    fn generate_ssao_kernel_data<const N: usize>() -> [Vec4; N] {
        use core::array::from_fn;
        let mut rng = rng();
        from_fn(|i| {
            // loop prevents non-finite vector after normalization
            let kernel = loop {
                let kernel = Vec4::new(
                    rng.random_range(-1.0..=1.0),
                    rng.random_range(-1.0..=1.0),
                    rng.random_range(0.0..=1.0),
                    0.0,
                ).truncate().try_normalize();

                if let Some(kernel) = kernel {
                    break kernel;
                }
            };
            let t = i as f32 / ((N as f32) - 1.0);
            (kernel * (0.1 + 0.9 * t * t)).extend(0.0)
        })
    }
}

impl PassNode for SsaoPassNode {
    fn run(
        &mut self,
        encoder: &mut CommandEncoder,
        _layers: &mut Layers,
        global_context: &mut GlobalContext,
    ) {
        if let Some(ssao_texture_view) = global_context.texture_view_resources.get(TextureViewKind::SSAO) {
            let ssao_texture = ssao_texture_view.texture();
            encoder.clear_texture(ssao_texture, &ImageSubresourceRange::default());
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("SSAO Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.ssao_compute_pipeline);
            compute_pass.set_bind_group(0, &self.camera_ssao_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.ssao_bind_group, &[]);
            let wg_x = (ssao_texture.size().width as f32 / 8.0).ceil() as u32;
            let wg_y = (ssao_texture.size().height as f32 / 8.0).ceil() as u32;
            compute_pass.dispatch_workgroups(wg_x, wg_y, 1);
        }
    }
}
