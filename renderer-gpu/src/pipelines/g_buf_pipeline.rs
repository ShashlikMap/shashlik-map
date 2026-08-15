use crate::global_context::GlobalContext;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::RenderPipeline;
use crate::vertex_attrs::GeneralInstanceInput;
use std::borrow::Cow;
use wesl::include_wesl;
use wgpu::TextureFormat::Rgba16Float;
use wgpu::{RenderPass, ShaderModuleDescriptor, ShaderSource, TextureFormat};

pub struct GBufPipeline {
    mesh_pipeline: MeshPipeline,
    render_pipeline: wgpu::RenderPipeline,
}

impl GBufPipeline {
    pub fn new(global_context: &GlobalContext) -> Self {
        let mesh_pipeline = MeshPipeline::new(global_context, false, false, false);
        let mut root_descriptor = mesh_pipeline.prepare(global_context);

        let g_buf_frag_shader_module =
            global_context
                .device()
                .create_shader_module(ShaderModuleDescriptor {
                    label: Some("g_buf_frag_shader"),
                    source: ShaderSource::Wgsl(Cow::from(include_wesl!("g_buf_frag_shader"))),
                });
        root_descriptor.label = Some("g_buffer_pipeline");
        let fragment = root_descriptor.fragment.as_mut().unwrap();
        fragment.module = g_buf_frag_shader_module;
        fragment.targets = vec![
            Some(wgpu::ColorTargetState {
                format: Rgba16Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: Rgba16Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
        ];
        fragment.entry_point = Some("fs_main_g_buf");
        root_descriptor.multisample.count = 1;
        // render pass for g buffer uses Depth24Plus but original descriptor Depth24PlusStencil8
        root_descriptor.depth_stencil.as_mut().unwrap().format = TextureFormat::Depth24Plus;
        let pipeline = root_descriptor.to_render_pipeline(global_context.device());
        Self {
            mesh_pipeline,
            render_pipeline: pipeline,
        }
    }
}

impl RenderPipeline<GeneralInstanceInput> for GBufPipeline {

    fn setup_render(&mut self, render_pass: &mut RenderPass, global_context: &GlobalContext) {
        render_pass.set_pipeline(&self.render_pipeline);
        self.mesh_pipeline.setup_render(render_pass, global_context);
    }
}
