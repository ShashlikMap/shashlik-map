use crate::draw_commands::MeshVertex;
use crate::global_context::GlobalContext;
use crate::pipelines::{IndirectInstancesLayout, OwnedFragmentState, OwnedRenderPipelineDescriptor, OwnedVertexState, RenderPipeline, WithSSAOTexture};
use crate::textures::{create_simple_texture, TextureData, SAMPLE_COUNT};
use crate::vertex_attrs::{GeneralInstanceInput, VertexAttrib};
use std::borrow::Cow;
use wesl::include_wesl;
use wgpu::{BindGroup, BindGroupLayout, BlendState, CompareFunction, ComputePass, DepthStencilState, Face, RenderPass, SamplerDescriptor, ShaderModuleDescriptor, ShaderSource, TextureFormat, TextureUsages, TextureView};
use wgpu_canvas::SHADOWS_ENABLED;

pub struct MeshPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub depth_bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
    depth_bind_group: BindGroup,
    depth_dummy_bind_group: BindGroup,
}

impl MeshPipeline {
    pub fn new(global_context: &GlobalContext) -> Self {
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
                    &global_context.shadow_map_depth_texture,
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

        MeshPipeline {
            bind_group_layout,
            depth_bind_group_layout,
            bind_group,
            depth_bind_group,
            depth_dummy_bind_group,
        }
    }
}

impl WithSSAOTexture for MeshPipeline {
    fn update_ssao_texture(&mut self, texture_view: &TextureView, global_context: &GlobalContext) {
        // TODO
    }
}

impl RenderPipeline for MeshPipeline {
    type InstanceInputType = GeneralInstanceInput;

    fn compute(&mut self, _compute_pass: &mut ComputePass, _global_context: &GlobalContext) {
    }

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &GlobalContext) {
        let mut mask = global_context.is_shadow_render as u32;
        if unsafe { SHADOWS_ENABLED } {
            mask |= 2;
        }
        render_pass.set_immediates(
            0,
            bytemuck::bytes_of(&mask),
        );
        render_pass.set_bind_group(0, &self.bind_group, &[]);

        if unsafe { !SHADOWS_ENABLED } ||
            global_context.is_shadow_render ||
            global_context.is_preview_render ||
            global_context.is_g_buffer_render {
            render_pass.set_bind_group(1, &self.depth_dummy_bind_group, &[]);
        } else {
            render_pass.set_bind_group(1, &self.depth_bind_group, &[]);
        }
    }

    fn prepare(&self, global_context: &GlobalContext) -> OwnedRenderPipelineDescriptor<'_> {
        let device = global_context.device();
        let config = global_context.config();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mesh Render Pipeline Layout"),
            bind_group_layouts: &[&self.bind_group_layout, &self.depth_bind_group_layout],
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
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                ..Default::default()
            },
            depth_stencil: Some({
                DepthStencilState {
                    format: TextureFormat::Depth24Plus,
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::Less,
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

    fn set_instance_bind_group_compute(&mut self, _compute_pass: &mut ComputePass, _instance_bind_group: &BindGroup, _instance_args_bind_group: &BindGroup) {
    }
    
    fn set_instance_bind_group_render(&mut self, _render_pass: &mut RenderPass, _instance_bind_group: &BindGroup) {
    }

    fn get_instances_layouts(&self) -> Option<IndirectInstancesLayout> {
        None
    }

    fn is_indirect(&self) -> bool {
        false
    }

    fn support_g_buf(&self) -> bool {
        true
    }
}
