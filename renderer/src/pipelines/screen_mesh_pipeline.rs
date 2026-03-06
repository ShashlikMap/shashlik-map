use crate::global_context::GlobalContext;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::{OwnedRenderPipelineDescriptor, RenderPipeline, WithTexture};
use crate::vertex_attrs::{MeshVertexWithUV, ShapeInstanceInput, TextInstanceInput, VertexAttrib};
use wgpu::{include_wgsl, BindGroup, BindGroupLayout, CompareFunction, RenderPass, TextureView};

pub struct ScreenMeshPipeline {
    mesh_pipeline: MeshPipeline,
    texture_bind_group_layout: BindGroupLayout,
    use_texture: bool,
}

impl ScreenMeshPipeline {
    pub fn new(global_context: &GlobalContext, use_texture: bool) -> Self {
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
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        Self {
            mesh_pipeline: MeshPipeline::new(global_context),
            texture_bind_group_layout,
            use_texture,
        }
    }
}

impl RenderPipeline for ScreenMeshPipeline {
    type InstanceInputType = ShapeInstanceInput;

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &GlobalContext) {
        self.mesh_pipeline.render(render_pass, global_context);
    }

    fn prepare(&mut self, global_context: &GlobalContext) -> OwnedRenderPipelineDescriptor<'_> {
        let device = global_context.device();
        let texture_pipeline_layout = if self.use_texture {
            Some(device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Texture Render Pipeline Layout"),
                    bind_group_layouts: &[
                        &self.mesh_pipeline.bind_group_layout,
                        &self.texture_bind_group_layout,
                    ],
                    ..Default::default()
                },
            ))
        } else {
            None
        };
        
        let mut mesh_descriptor = self.mesh_pipeline.prepare(global_context);
        mesh_descriptor.layout = texture_pipeline_layout;
        
        let mut stencil = mesh_descriptor.depth_stencil.unwrap();
        stencil.depth_compare = CompareFunction::Always;
        stencil.depth_write_enabled = false;
        mesh_descriptor.depth_stencil = Some(stencil);

        let shader_module =
            device.create_shader_module(include_wgsl!("../shaders/screen_mesh_shader.wgsl"));

        let vertex = &mut mesh_descriptor.vertex;
        vertex.module = shader_module.to_owned();
        vertex.buffers = vec![MeshVertexWithUV::desc(), TextInstanceInput::desc()];

        let fragment = mesh_descriptor.fragment.as_mut().unwrap();
        fragment.module = shader_module;
        if self.use_texture {
            fragment.entry_point = Some("fs_main_textured");
        }

        mesh_descriptor.primitive.cull_mode = None;

        mesh_descriptor
    }

    fn set_instance_bind_group(&mut self, _render_pass: &mut RenderPass, _instance_bind_group: &BindGroup) {
    }

    fn get_instances_layout(&self) -> Option<&BindGroupLayout> {
        None
    }
}

impl WithTexture for ScreenMeshPipeline {
    fn create_texture_bind_group(
        &mut self,
        texture_view: &TextureView,
        global_context: &GlobalContext,
    ) -> BindGroup {
        let device = global_context.device();
        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        })
    }
}
