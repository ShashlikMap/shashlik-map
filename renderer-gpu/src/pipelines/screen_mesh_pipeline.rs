use crate::global_context::GlobalContext;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::{OwnedRenderPipelineDescriptor, RenderPipeline};
use crate::vertex_attrs::{MeshVertexWithUV, ScreenShapeInstanceInput, VertexAttrib};
use wesl::Compiler;
use wesl::resolver::PackageResolver;
use wgpu::{BindGroup, BindGroupLayout, BindingType, CompareFunction, Device, FilterMode, MipmapFilterMode, RenderPass, SamplerBindingType, SamplerDescriptor, ShaderModuleDescriptor, ShaderSource, StencilFaceState, TextureFormat, TextureUsages, TextureView};

pub(crate) struct ScreenMeshPipeline {
    mesh_pipeline: MeshPipeline,
    texture_type_and_bind_group: Option<BindGroup>,
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
    Cube
}

impl TextureType {
    fn get_shader_type(&self) -> &str {
        match self {
            TextureType::GeneralRgba => "GENERAL_RGBA",
            TextureType::GeneralRFloat => "GENERAL_R_FLOAT",
            TextureType::Depth => "DEPTH",
            TextureType::Cube => "CUBE"
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
        } else if matches!(texture_format, TextureFormat::Astc {..}) ||
            matches!(texture_format, TextureFormat::Bc7RgbaUnormSrgb) {
            // fyi, Astc/Bc7RgbaUnormSrgb doesn't mean this is a cube texture, but OK for our use case now
            TextureType::Cube
        } else {
            TextureType::GeneralRgba
        }
    }
}

pub struct TextureInfo {
    pub use_texture: bool,
    pub filterable: bool,
    pub fs_shader: &'static str,
}

impl TextureInfo {
    fn sample_binding_type(&self) -> SamplerBindingType {
        if self.filterable { SamplerBindingType::Filtering } else { SamplerBindingType::NonFiltering }
    }
    fn filter_modes(&self) -> (FilterMode, MipmapFilterMode) {
        if self.filterable {
            (wgpu::FilterMode::Linear, MipmapFilterMode::Linear)
        } else {
            (wgpu::FilterMode::Nearest, MipmapFilterMode::Nearest)
        }
    }

    fn create_texture_bind_group_layout(&self, device: &Device, texture_view: Option<&TextureView>) -> Option<BindGroupLayout> {
        if !self.use_texture {
            return None;
        }
        let mut groups = vec![];
        if let Some(texture_view) = texture_view {
            let ty = match TextureType::get_texture_type(texture_view) {
                TextureType::Depth => {
                    wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Depth,
                    }
                },
                TextureType::Cube => {
                    wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    }
                }
                _ => {
                    wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: self.filterable,
                        },
                    }
                }
            };
            groups.push(wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty,
                count: None,
            });
        }

        groups.push(wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(self.sample_binding_type()),
            count: None,
        });

        Some(device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: groups.as_slice(),
            label: Some("texture_bind_group_layout"),
        }))
    }
}

impl ScreenMeshPipeline {
    pub fn new(
        global_context: &GlobalContext,
        texture_info: TextureInfo,
        texture_view: Option<&TextureView>,
        read_stencil: bool,
    ) -> Self {
        let device = global_context.device();

        let mut result = Self {
            mesh_pipeline: MeshPipeline::new(global_context, false, false, false),
            texture_type_and_bind_group: None,
            texture_info,
            read_stencil,
            pipeline: None,
        };

        let texture_type = texture_view.map(|tv| {
            TextureType::get_texture_type(tv)
        });
        let texture_bind_group_layout = result.texture_info.create_texture_bind_group_layout(device, texture_view);
        result.pipeline = Some(
            result
                .prepare(global_context, texture_type, texture_bind_group_layout.as_ref())
                .to_render_pipeline(global_context.device()),
        );
        result.set_texture_view(texture_view, device, texture_bind_group_layout);
        result
    }

    fn prepare(&self, global_context: &GlobalContext, texture_type: Option<TextureType>, texture_bind_group_layout: Option<&BindGroupLayout>) -> OwnedRenderPipelineDescriptor<'_> {
        let mut mesh_descriptor = self.mesh_pipeline.prepare(global_context);
        mesh_descriptor.label = Some("Screen Mesh Pipeline");
        let device = global_context.device();

        if let Some(texture_bind_group_layout) = texture_bind_group_layout {
            mesh_descriptor.layout = Some(device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Texture Render Pipeline Layout"),
                    bind_group_layouts: &[
                        Some(&self.mesh_pipeline.bind_group_layout),
                        Some(texture_bind_group_layout),
                    ],
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

        // TODO Some cache to prevent recompilation.
        let mut wesl_resolver = PackageResolver::new();
        wesl_resolver.add_package(&crate::shaders::PACKAGE);
        let mut wesl_compiler = Compiler::default().with_resolver(wesl_resolver);
        let tex_type_shader_type = texture_type.as_ref().map(|t_type|
            t_type.get_shader_type()).unwrap_or(TextureType::GeneralRgba.get_shader_type());
        wesl_compiler.options.features.set(tex_type_shader_type, true);
        let wesl_shader = wesl_compiler
            .compile_module(&"shaders::screen_mesh_shader".parse().unwrap())
            .unwrap_or_else(|e| panic!("failed to compile screen_mesh_shader: {e}")).to_string();
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("screen_mesh_shader"),
            source: ShaderSource::Wgsl(wesl_shader.into()),
        });

        let vertex = &mut mesh_descriptor.vertex;
        vertex.module = shader_module.to_owned();
        vertex.buffers = vec![MeshVertexWithUV::desc(), ScreenShapeInstanceInput::desc()];

        let fragment = mesh_descriptor.fragment.as_mut().unwrap();
        fragment.module = shader_module;
        if self.texture_info.use_texture {
            vertex.entry_point = Some("vs_main");
            fragment.entry_point = Some(self.texture_info.fs_shader);
        }
        if !self.texture_info.fs_shader.is_empty() {
            fragment.entry_point = Some(self.texture_info.fs_shader);
        }

        mesh_descriptor.primitive.cull_mode = None;

        mesh_descriptor
    }

    fn set_texture_view(&mut self, texture_view: Option<&TextureView>, device: &wgpu::Device, texture_bind_group_layout: Option<BindGroupLayout>) {
        if let (Some(texture_view), Some(texture_bind_group_layout)) = (texture_view, texture_bind_group_layout) {
            // fyi, in future we may need to cache bind group here for more dynamic behavior
            // now, it'll be called only in MainPassNode constructor for new render configuration
            println!(
                "Created texture view: {:?} for ScreenMeshPipeline",
                texture_view
            );
            self.texture_type_and_bind_group = Some(self.create_texture_bind_group(texture_view, device, texture_bind_group_layout));
        }
    }

    fn create_texture_bind_group(
        &self,
        texture_view: &TextureView,
        device: &wgpu::Device,
        texture_bind_group_layout: BindGroupLayout
    ) -> BindGroup {
        let filter_modes = self.texture_info.filter_modes();
        let diffuse_sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: filter_modes.0,
            min_filter: filter_modes.0,
            mipmap_filter: filter_modes.1,
            ..Default::default()
        });
        let entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
            }];

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
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
        if let Some(texture_bind_group) = self.texture_type_and_bind_group.as_ref() {
            render_pass.set_bind_group(1, texture_bind_group, &[]);
        }
    }
}
