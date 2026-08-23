use crate::global_context::GlobalContext;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::{OwnedRenderPipelineDescriptor, RenderPipeline};
use crate::textures::{TextureData, create_simple_texture};
use crate::vertex_attrs::{MeshVertexWithUV, ScreenShapeInstanceInput, VertexAttrib};
use std::borrow::Cow;
use wesl::include_wesl;
use wgpu::{BindGroup, BindGroupLayout, BindingType, CompareFunction, FilterMode, RenderPass, SamplerBindingType, SamplerDescriptor, ShaderModuleDescriptor, ShaderSource, StencilFaceState, TextureFormat, TextureUsages, TextureView};

pub struct ScreenMeshPipeline {
    mesh_pipeline: MeshPipeline,
    texture_bind_group_layout: BindGroupLayout,
    texture_type_and_bind_group: Option<(TextureType, BindGroup)>,
    texture_info: TextureInfo,
    read_stencil: bool,
    pipeline: Option<wgpu::RenderPipeline>,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
enum TextureType {
    GeneralRgba,
    GeneralRFloat,
    Depth,
}

pub struct TextureInfo {
    pub use_texture: bool,
    pub filterable: bool,
    pub vs_shader: Option<&'static str>,
    pub fs_shader: &'static str,
}

impl TextureInfo {

    fn sample_binding_type(&self) -> SamplerBindingType {
        if self.filterable { SamplerBindingType::Filtering } else { SamplerBindingType::NonFiltering }
    }
    fn filter_mode(&self) -> FilterMode {
        if self.filterable { wgpu::FilterMode::Linear } else { wgpu::FilterMode::Nearest }
    }
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
                        ty: BindingType::Sampler(texture_info.sample_binding_type()),
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
            texture_type_and_bind_group: None,
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

        let mut depth_stencil = mesh_descriptor.depth_stencil.unwrap();
        if self.read_stencil {
            depth_stencil.stencil = wgpu::StencilState {
                front: StencilFaceState::IGNORE,
                back: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::NotEqual,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                read_mask: 0xFF,
                write_mask: 0x00,
            };
        } else {
            depth_stencil.depth_compare = Some(CompareFunction::Always);
            depth_stencil.depth_write_enabled = Some(false);
        }
        mesh_descriptor.depth_stencil = Some(depth_stencil);

        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("screen_mesh_shader"),
            source: ShaderSource::Wgsl(Cow::from(include_wesl!("screen_mesh_shader"))),
        });

        let vertex = &mut mesh_descriptor.vertex;
        vertex.module = shader_module.to_owned();
        vertex.buffers = vec![MeshVertexWithUV::desc(), ScreenShapeInstanceInput::desc()];

        let fragment = mesh_descriptor.fragment.as_mut().unwrap();
        fragment.module = shader_module;
        if self.texture_info.use_texture {
            vertex.entry_point = self.texture_info.vs_shader.or(Some("vs_main"));
            fragment.entry_point = Some(self.texture_info.fs_shader);
        }

        mesh_descriptor.primitive.cull_mode = None;

        mesh_descriptor
    }

    pub fn set_texture_view(&mut self, texture_view: Option<&TextureView>, device: &wgpu::Device) {
        if let Some(texture_view) = texture_view {
            // fyi, in future we may need to cache bind group here for more dynamic behavior
            // now, it'll be called only in MainPassNode constructor for new render configuration
            println!(
                "Created texture view: {:?} for ScreenMeshPipeline",
                texture_view
            );
            let texture_type = Self::get_texture_type(texture_view);
            self.texture_type_and_bind_group = Some((texture_type, self.create_texture_bind_group(texture_view, device)));
        }
    }

    fn get_texture_type(texture_view: &TextureView) -> TextureType {
        let texture = texture_view.texture();
        let texture_format = texture.format();
        let texture_usage = texture.usage();
        if texture_format.is_depth_stencil_format() {
            TextureType::Depth
        } else if texture_format == TextureFormat::R16Float
            || texture_format == TextureFormat::R32Float
        {
            TextureType::GeneralRFloat
        } else if texture_format == TextureFormat::Rgba16Float {
            if texture_usage.contains(TextureUsages::STORAGE_BINDING) {
                TextureType::GeneralRFloat
            } else {
                TextureType::GeneralRgba
            }
        } else {
            TextureType::GeneralRgba
        }
    }

    fn create_texture_bind_group(
        &self,
        texture_view: &TextureView,
        device: &wgpu::Device,
    ) -> BindGroup {
        let diffuse_sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: self.texture_info.filter_mode(),
            min_filter: self.texture_info.filter_mode(),
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

impl RenderPipeline<ScreenShapeInstanceInput> for ScreenMeshPipeline {

    fn setup_render(&mut self, render_pass: &mut RenderPass, global_context: &GlobalContext) {
        self.mesh_pipeline.setup_render(render_pass, global_context);

        if let Some(pipeline) = self.pipeline.as_ref() {
            render_pass.set_pipeline(pipeline);
            if self.read_stencil {
                render_pass.set_stencil_reference(1);
            }
        }
        if let Some((texture_type, texture_bind_group)) = self.texture_type_and_bind_group.as_ref() {
            render_pass.set_immediates(0, bytemuck::bytes_of(&(*texture_type as u32)));
            render_pass.set_bind_group(1, texture_bind_group, &[]);
        }
    }
}
