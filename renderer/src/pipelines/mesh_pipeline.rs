use crate::pipelines::{
    OwnedFragmentState, OwnedRenderPipelineDescriptor, OwnedVertexState, RenderPipeline,
};
use crate::vertex_attrs::{GeneralInstanceInput, VertexAttrib};
use crate::global_context::GlobalContext;
use wgpu::{
    include_wgsl, BindGroup, BindGroupLayout, BlendState, CompareFunction, DepthStencilState
    , Face, RenderPass, TextureFormat,
};
use crate::draw_commands::MeshVertex;
use crate::textures::SAMPLE_COUNT;

pub struct MeshPipeline {
    pub bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
}

impl MeshPipeline {
    pub fn new(global_context: &GlobalContext) -> Self {
        let device = global_context.device();
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("mesh_pipeline_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: global_context
                    .view_projection
                    .uniform_buffer
                    .as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });
        MeshPipeline {
            bind_group_layout,
            bind_group,
        }
    }
}

impl RenderPipeline for MeshPipeline {
    type InstanceInputType = GeneralInstanceInput;

    fn render(&mut self, render_pass: &mut RenderPass, _global_context: &GlobalContext) {
        render_pass.set_bind_group(0, &self.bind_group, &[]);
    }

    fn prepare(&'_ self, global_context: &GlobalContext) -> OwnedRenderPipelineDescriptor<'_> {
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
}
