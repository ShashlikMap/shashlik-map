use crate::global_context::GlobalContext;
use crate::pipelines::RenderPipeline;
use crate::textures::SAMPLE_COUNT;
use crate::vertex_attrs::GeneralInstanceInput;
use std::borrow::Cow;
use wesl::include_wesl;
use wgpu::{
    BlendState, DepthBiasState, DepthStencilState, RenderPass, ShaderModuleDescriptor,
    ShaderSource, StencilState, TextureFormat,
};

pub struct XRealMeshShaderPipeline {
    pipeline: Option<wgpu::RenderPipeline>,
}

impl XRealMeshShaderPipeline {
    pub fn new(global_context: &GlobalContext, enabled: bool) -> Self {
        if !enabled {
            return XRealMeshShaderPipeline { pipeline: None };
        }
        let device = global_context.device();
        let config = global_context.config();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("X Real MeshShader Render Pipeline Layout"),
            immediate_size: 4,
            ..Default::default()
        });
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("x_real_mesh_shader"),
            source: ShaderSource::Wgsl(Cow::from(include_wesl!("x_real_mesh_shader"))),
        });
        let pipeline = device.create_mesh_pipeline(&wgpu::MeshPipelineDescriptor {
            label: Some("X Real Mesh Shader Pipeline"),
            layout: Some(&pipeline_layout),
            task: None,
            mesh: wgpu::MeshState {
                module: &shader_module,
                entry_point: Some("ms_main"),
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                // targets: &[Some(config.view_formats[0].into())],
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some({
                DepthStencilState {
                    format: TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: Some(false),
                    depth_compare: None,
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                }
            }),
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });
        XRealMeshShaderPipeline {
            pipeline: Some(pipeline),
        }
    }
}

impl RenderPipeline<GeneralInstanceInput> for XRealMeshShaderPipeline {
    fn setup_render(&mut self, render_pass: &mut RenderPass, _global_context: &GlobalContext) {
        if let Some(pipeline) = &self.pipeline {
            render_pass.set_pipeline(pipeline);
            render_pass.draw_mesh_tasks(1, 1, 1);
        }
    }

    /// General mesh rendering is disabled for now for this pipeline
    fn is_mesh_rendering_enabled(&self) -> bool {
        false
    }
}
