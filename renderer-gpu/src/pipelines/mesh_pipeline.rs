use crate::global_context::GlobalContext;
use crate::global_context::GlobalRenderStep::{MainStep, ShadowStep};
use crate::pipelines::{OwnedFragmentState, OwnedRenderPipelineDescriptor, OwnedVertexState, RenderPipeline};
use crate::texture_view_resources::TextureViewKind;
use crate::textures::{SAMPLE_COUNT, TextureData, create_simple_texture};
use crate::vertex_attrs::{GeneralInstanceInput, VertexAttrib};
use renderer_common::geometry_data::MeshVertex;
use std::borrow::Cow;
use wesl::include_wesl;
use wgpu::{BindGroup, BindGroupLayout, BlendState, CompareFunction, DepthStencilState, Face, RenderPass, SamplerDescriptor, ShaderModuleDescriptor, ShaderSource, TextureFormat, TextureUsages};

pub struct MeshPipeline {
    pub bind_group_layout: BindGroupLayout,
    depth_bind_group_layout: Option<BindGroupLayout>,
    pub bind_group: BindGroup,
    depth_bind_group: BindGroup,
    depth_dummy_bind_group: BindGroup,
}

impl MeshPipeline {
    pub fn new(global_context: &GlobalContext, enable_depth_group: bool) -> Self {
        let device = global_context.device();
        let entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT | wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &entries,
            label: Some("mesh_pipeline_group_layout"),
        });
        
        let entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: global_context
                .view_projection
                .uniform_buffer
                .as_entire_binding(),
        }];
        
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &entries,
            label: Some("mesh_pipeline_bind_group"),
        });

        let depth_bind_group_entries = vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Depth,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            },
        ];

        let depth_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &depth_bind_group_entries,
            label: Some("mesh_pipeline_depth_group_layout"),
        });

        let depth_sampler = device.create_sampler(&SamplerDescriptor {
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
                format: TextureFormat::Depth32Float,
            },
            device,
        );

        let mut depth_entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(
                    &global_context.texture_view_resources.get_or_unwrap(TextureViewKind::ShadowMapDepth),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&depth_sampler),
            },
        ];

        let depth_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &depth_bind_group_layout,
            entries: &depth_entries,
            label: Some("mesh_pipeline_depth_bind_group"),
        });

        depth_entries[0] = wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(
                &dummy_texture,
            ),
        };
        let depth_dummy_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &depth_bind_group_layout,
            entries: &depth_entries,
            label: Some("mesh_pipeline_dummy_depth_bind_group"),
        });
        let depth_bind_group_layout= if enable_depth_group { Some(depth_bind_group_layout) } else { None };
        MeshPipeline {
            bind_group_layout,
            depth_bind_group_layout,
            bind_group,
            depth_bind_group,
            depth_dummy_bind_group,
        }
    }
}

impl RenderPipeline for MeshPipeline {
    type InstanceInputType = GeneralInstanceInput;

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &GlobalContext) {
        let mut mask = global_context.check_render_step(ShadowStep) as u32;
        if global_context.is_shadow_mapping_enabled() {
            mask |= 2;
        }
        render_pass.set_immediates(
            0,
            bytemuck::bytes_of(&mask),
        );
        render_pass.set_bind_group(0, &self.bind_group, &[]);


        if self.depth_bind_group_layout.is_some() {
            if !global_context.is_shadow_mapping_enabled() ||
                !global_context.check_render_step(MainStep) {
                render_pass.set_bind_group(1, &self.depth_dummy_bind_group, &[]);
            } else {
                render_pass.set_bind_group(1, &self.depth_bind_group, &[]);
            }
        }
    }

    fn prepare(&self, global_context: &GlobalContext) -> OwnedRenderPipelineDescriptor<'_> {
        let device = global_context.device();
        let config = global_context.config();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mesh Render Pipeline Layout"),
            bind_group_layouts: &[Some(&self.bind_group_layout), self.depth_bind_group_layout.as_ref()],
            immediate_size: 4,
            ..Default::default()
        });
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("mesh_shader"),
            source: ShaderSource::Wgsl(Cow::from(include_wesl!("mesh_shader"))),
        });
        OwnedRenderPipelineDescriptor {
            label: Some("Mesh Render Pipeline"),
            layout: Some(pipeline_layout),
            vertex: OwnedVertexState {
                module: shader_module.to_owned(),
                entry_point: Some("vs_main"),
                buffers: vec![MeshVertex::desc(), GeneralInstanceInput::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(OwnedFragmentState {
                module: shader_module.to_owned(),
                entry_point: Some("fs_main"),
                targets: vec![Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(Face::Back),
                ..Default::default()
            },
            depth_stencil: Some({
                DepthStencilState {
                    format: TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }
            }),
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        }
    }

    fn is_indirect(&self) -> bool {
        false
    }

    fn support_g_buf(&self) -> bool {
        true
    }
}
