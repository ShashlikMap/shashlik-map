use crate::draw_commands::MeshVertex;
use crate::global_context::GlobalContext;
use crate::pipelines::{OwnedFragmentState, OwnedRenderPipelineDescriptor, OwnedVertexState, RenderPipeline, WithSSAOTexture};
use crate::textures::SAMPLE_COUNT;
use crate::vertex_attrs::{GeneralInstanceInput, VertexAttrib};
use wgpu::{include_wgsl, BindGroup, BindGroupLayout, BlendState, CompareFunction, ComputePass, DepthStencilState, Face, RenderPass, TextureFormat, TextureView};

pub struct MeshPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
    ortho_bind_group: BindGroup,
}

impl MeshPipeline {
    pub fn new(global_context: &GlobalContext) -> Self {
        let device = global_context.device();
        let entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::COMPUTE,
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

        let ortho_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &vec![wgpu::BindGroupEntry {
                binding: 0,
                resource: global_context
                    .view_projection
                    .ortho_uniform_buffer
                    .as_entire_binding(),
            }],
            label: Some("mesh_pipeline_ortho_bind_group"),
        });
        MeshPipeline {
            bind_group_layout,
            bind_group,
            ortho_bind_group
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
        if global_context.is_pre_depth_render {
            render_pass.set_bind_group(0, &self.ortho_bind_group, &[]);
        } else {
            render_pass.set_bind_group(0, &self.bind_group, &[]);
        }
    }

    fn prepare(&self, global_context: &GlobalContext) -> OwnedRenderPipelineDescriptor<'_> {
        let device = global_context.device();
        let config = global_context.config();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mesh Render Pipeline Layout"),
            bind_group_layouts: &[&self.bind_group_layout],
            ..Default::default()
        });
        let shader_module =
            device.create_shader_module(include_wgsl!("../shaders/mesh_shader.wgsl"));
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
                    format: TextureFormat::Depth32Float,
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

    fn get_instances_layouts(&self) -> Option<(&BindGroupLayout, &BindGroupLayout)> {
        None
    }

    fn is_indirect(&self) -> bool {
        false
    }
}
