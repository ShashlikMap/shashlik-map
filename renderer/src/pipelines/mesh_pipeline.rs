use crate::draw_commands::MeshVertex;
use crate::global_context::GlobalContext;
use crate::pipelines::{OwnedFragmentState, OwnedRenderPipelineDescriptor, OwnedVertexState, RenderPipeline, WithSSAOTexture};
use crate::textures::SAMPLE_COUNT;
use crate::vertex_attrs::{GeneralInstanceInput, VertexAttrib};
use wgpu::{include_wgsl, BindGroup, BindGroupLayout, BlendState, CompareFunction, ComputePass, DepthStencilState, Face, RenderPass, TextureFormat, TextureView};

pub struct MeshPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
}

impl MeshPipeline {
    pub fn new(global_context: &GlobalContext, ssao_enabled: bool) -> Self {
        let device = global_context.device();
        let mut entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];
        if ssao_enabled {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            });
        }
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &entries,
            label: Some("mesh_pipeline_group_layout"),
        });
        
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: global_context
                .view_projection
                .uniform_buffer
                .as_entire_binding(),
        }];
        if ssao_enabled {
            entries.push(wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&global_context.ssao_texture),
            });
        }

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &entries,
            label: Some("camera_bind_group"),
        });
        MeshPipeline {
            bind_group_layout,
            bind_group,
        }
    }
}

impl WithSSAOTexture for MeshPipeline {
    fn update_ssao_texture(&mut self, texture_view: &TextureView, global_context: &GlobalContext) {
        // let device = global_context.device();
        // self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     layout: &self.bind_group_layout,
        //     entries: &[wgpu::BindGroupEntry {
        //         binding: 0,
        //         resource: global_context
        //             .view_projection
        //             .uniform_buffer
        //             .as_entire_binding(),
        //     }],
        //     label: Some("camera_bind_group"),
        // });
    }
}

impl RenderPipeline for MeshPipeline {
    type InstanceInputType = GeneralInstanceInput;

    fn compute(&mut self, _compute_pass: &mut ComputePass, _global_context: &GlobalContext) {
    }

    fn render(&mut self, render_pass: &mut RenderPass, _global_context: &GlobalContext) {
        render_pass.set_bind_group(0, &self.bind_group, &[]);
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

    fn get_instances_layouts(&self) -> Option<(&BindGroupLayout, &BindGroupLayout)> {
        None
    }

    fn is_indirect(&self) -> bool {
        false
    }
}
