use crate::global_context::GlobalContext;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::{OwnedRenderPipelineDescriptor, RenderPipeline};
use crate::textures::{TextureData, create_simple_texture};
use crate::vertex_attrs::{MeshVertexWithUV, ShapeInstanceInput, TextInstanceInput, VertexAttrib};
use std::borrow::Cow;
use wesl::include_wesl;
use wgpu::{
    BindGroup, BindGroupLayout, CompareFunction, RenderPass, SamplerDescriptor,
    ShaderModuleDescriptor, ShaderSource, TextureFormat, TextureUsages, TextureView,
};

pub struct ScreenMeshPipeline {
    mesh_pipeline: MeshPipeline,
    texture_bind_group_layout: BindGroupLayout,
    texture_bind_group: Option<BindGroup>,
    texture_info: TextureInfo,
    read_stencil: bool,
    pipeline: Option<wgpu::RenderPipeline>,
}

pub struct TextureInfo {
    pub use_texture: bool,
    pub filterable: bool,
    pub vs_shader: Option<&'static str>,
    pub fs_shader: &'static str,
}

impl ScreenMeshPipeline {
    pub fn new(
        global_context: &GlobalContext,
        texture_info: TextureInfo,
        read_stencil: bool,
    ) -> Self {
        let device = global_context.device();

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float {
                                filterable: texture_info.filterable,
                            },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Depth,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let mut result = Self {
            mesh_pipeline: MeshPipeline::new(global_context, false, false, false),
            texture_bind_group_layout,
            texture_bind_group: None,
            texture_info,
            read_stencil,
            pipeline: None,
        };

        result.pipeline = Some(
            result
                .prepare(global_context)
                .to_render_pipeline(global_context.device()),
        );
        result
    }

    pub fn set_texture_view(&mut self, texture_view: Option<&TextureView>, device: &wgpu::Device) {
        if let Some(texture_info) = texture_view {
            // fyi, in future we may need to cache bind group here for more dynamic behavior
            // now, it'll be called only in MainPassNode constructor
            println!(
                "Created texture view: {:?} for ScreenMeshPipeline",
                texture_info
            );
            self.texture_bind_group = Some(self.create_texture_bind_group(texture_info, device));
        }
    }

    fn create_texture_bind_group(
        &self,
        texture_view: &TextureView,
        device: &wgpu::Device,
    ) -> BindGroup {
        let diffuse_sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let sampler_compare = device.create_sampler(&SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(CompareFunction::GreaterEqual),
            ..Default::default()
        });
        let dummy_texture = create_simple_texture(
            TextureData {
                sample_count: 1,
                size: (1, 1),
                usage: TextureUsages::TEXTURE_BINDING,
                format: TextureFormat::Rgba16Float,
            },
            device,
        );
        let dummy_depth_texture = create_simple_texture(
            TextureData {
                sample_count: 1,
                size: (1, 1),
                usage: TextureUsages::TEXTURE_BINDING,
                format: TextureFormat::Depth32Float,
            },
            device,
        );
        let mut entries: Vec<wgpu::BindGroupEntry> = vec![];
        if texture_view.texture().format().is_depth_stencil_format() {
            entries.push(wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&dummy_texture),
            });
            entries.push(wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(texture_view),
            });
        } else {
            entries.push(wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            });
            entries.push(wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&dummy_depth_texture),
            });
        }
        entries.push(wgpu::BindGroupEntry {
            binding: 2,
            resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 3,
            resource: wgpu::BindingResource::Sampler(&sampler_compare),
        });

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_bind_group_layout,
            entries: &entries,
            label: Some("texture_bind_group"),
        })
    }
}

impl RenderPipeline for ScreenMeshPipeline {
    type InstanceInputType = ShapeInstanceInput;

    fn setup_render(&mut self, render_pass: &mut RenderPass, global_context: &GlobalContext) {
        if let Some(pipeline) = self.pipeline.as_ref() {
            render_pass.set_pipeline(pipeline);
            if self.read_stencil {
                render_pass.set_stencil_reference(1);
            }
        }

        if let Some(texture_bind_group) = self.texture_bind_group.as_ref() {
            render_pass.set_bind_group(1, texture_bind_group, &[]);
        }

        self.mesh_pipeline.setup_render(render_pass, global_context);
    }

    fn prepare(&self, global_context: &GlobalContext) -> OwnedRenderPipelineDescriptor<'_> {
        let mut mesh_descriptor = self.mesh_pipeline.prepare(global_context);
        mesh_descriptor.label = Some("Screen Mesh Pipeline");
        let device = global_context.device();

        if self.texture_info.use_texture {
            mesh_descriptor.layout = Some(device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Texture Render Pipeline Layout"),
                    bind_group_layouts: &[
                        Some(&self.mesh_pipeline.bind_group_layout),
                        Some(&self.texture_bind_group_layout),
                    ],
                    immediate_size: 4,
                    ..Default::default()
                },
            ));
        }

        let mut stencil = mesh_descriptor.depth_stencil.unwrap();
        stencil.depth_compare = Some(CompareFunction::Always);
        stencil.depth_write_enabled = Some(false);
        mesh_descriptor.depth_stencil = Some(stencil);

        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("screen_mesh_shader"),
            source: ShaderSource::Wgsl(Cow::from(include_wesl!("screen_mesh_shader"))),
        });

        let vertex = &mut mesh_descriptor.vertex;
        vertex.module = shader_module.to_owned();
        vertex.buffers = vec![MeshVertexWithUV::desc(), TextInstanceInput::desc()];

        let fragment = mesh_descriptor.fragment.as_mut().unwrap();
        fragment.module = shader_module;
        if self.texture_info.use_texture {
            vertex.entry_point = self.texture_info.vs_shader.or(Some("vs_main"));
            fragment.entry_point = Some(self.texture_info.fs_shader);
        }

        mesh_descriptor.primitive.cull_mode = None;

        mesh_descriptor
    }

    fn is_indirect(&self) -> bool {
        false
    }
}
