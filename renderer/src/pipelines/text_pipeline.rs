use crate::global_context::GlobalContext;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::{OwnedRenderPipelineDescriptor, RenderPipeline};
use crate::text::glyph_tesselator::MeshVertexWithUV;
use crate::vertex_attrs::{ShapeInstanceInput, TextInstanceInput, VertexAttrib};
use wgpu::{include_wgsl, BindGroup, BindGroupLayout, CompareFunction, RenderPass, TextureView};

pub struct TextPipeline {
    mesh_pipeline: MeshPipeline,
    texture_bind_group_layout: Option<BindGroupLayout>,
    texture_bind_group: Option<BindGroup>,
}

impl TextPipeline {
    pub fn new(global_context: &GlobalContext, rt_texture_view: Option<&TextureView>) -> Self {
        let device = global_context.device();

        if let Some(rt_texture_view) = rt_texture_view.as_ref() {
            let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..Default::default()
            });

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

            let texture_bind_group = device.create_bind_group(
                &wgpu::BindGroupDescriptor {
                    layout: &texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&rt_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                        }
                    ],
                    label: Some("diffuse_bind_group"),
                }
            );

            Self {
                mesh_pipeline: MeshPipeline::new(global_context),
                texture_bind_group_layout: Some(texture_bind_group_layout),
                texture_bind_group: Some(texture_bind_group)
            }
        } else {
            Self {
                mesh_pipeline: MeshPipeline::new(global_context),
                texture_bind_group_layout: None,
                texture_bind_group: None,
            }
        }
    }
}

impl RenderPipeline for TextPipeline {
    type InstanceInputType = ShapeInstanceInput;

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &GlobalContext) {
        self.mesh_pipeline.render(render_pass, global_context);
        if let Some(texture_bind_group) = self.texture_bind_group.as_ref() {
            render_pass.set_bind_group(1, texture_bind_group, &[]);
        }
    }

    fn prepare(&self, global_context: &GlobalContext) -> OwnedRenderPipelineDescriptor<'_> {
        let mut mesh_descriptor = self.mesh_pipeline.prepare(global_context);

        let device = global_context.device();

        if let Some(texture_bind_group_layout) = self.texture_bind_group_layout.as_ref() {
            mesh_descriptor.layout = Some(device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Texture Render Pipeline Layout"),
                bind_group_layouts: &[&self.mesh_pipeline.bind_group_layout, &texture_bind_group_layout],
                ..Default::default()
            }));
        }

        let mut stencil = mesh_descriptor.depth_stencil.unwrap();
        stencil.depth_compare = CompareFunction::Always;
        stencil.depth_write_enabled = false;
        mesh_descriptor.depth_stencil = Some(stencil);

        let shader_module =
            device.create_shader_module(include_wgsl!("../shaders/text_shader.wgsl"));

        let vertex = &mut mesh_descriptor.vertex;
        vertex.module = shader_module.to_owned();
        vertex.buffers = vec![MeshVertexWithUV::desc(), TextInstanceInput::desc()];
        let fragment = &mut mesh_descriptor.fragment.as_mut().unwrap();
        fragment.module = shader_module;

        mesh_descriptor.primitive.cull_mode = None;

        mesh_descriptor
    }
}
