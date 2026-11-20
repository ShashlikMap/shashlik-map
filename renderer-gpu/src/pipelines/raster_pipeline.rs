use crate::global_context::GlobalContext;
use crate::textures::SAMPLE_COUNT;
use crate::vertex_attrs::{RasterInstance, RasterVertex, VertexAttrib};
use crate::DEPTH_STENCIL_TEX_FORMAT;
use wesl_core::include_wesl;
use wgpu::{BindGroup, BindGroupLayout, BlendState, CompareFunction, DepthStencilState, RenderPass, ShaderModuleDescriptor, ShaderSource};

/// Textured world space tiles (e.g. rain radar) drawn on the ground.
pub(crate) struct RasterPipeline {
    pipeline: wgpu::RenderPipeline,
    camera_bind_group: BindGroup,
}

impl RasterPipeline {
    pub fn new(global_context: &GlobalContext, texture_bind_group_layout: &BindGroupLayout) -> Self {
        let device = global_context.device();

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("raster_camera_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("raster_camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: global_context.view_projection.uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Raster Render Pipeline Layout"),
            bind_group_layouts: &[Some(&camera_bind_group_layout), Some(texture_bind_group_layout)],
            ..Default::default()
        });
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("raster_shader"),
            source: ShaderSource::Wgsl(include_wesl!("raster_shader").into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Raster Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[Some(RasterVertex::desc()), Some(RasterInstance::desc())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: global_context.config().format,
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            // Globe back side is discarded in the shader, so no culling (winding doesn't matter)
            primitive: wgpu::PrimitiveState::default(),
            // Same as ground shapes: drawn in order, buildings will be drawn on top
            depth_stencil: Some(DepthStencilState {
                format: DEPTH_STENCIL_TEX_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            cache: None,
            multiview_mask: None,
        });

        Self { pipeline, camera_bind_group }
    }

    pub fn setup_render(&self, render_pass: &mut RenderPass, _global_context: &GlobalContext) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
    }
}
