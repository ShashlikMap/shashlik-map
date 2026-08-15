use crate::global_context::GlobalContext;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::{OwnedRenderPipelineDescriptor, OwnedVertexState, RenderPipeline};
use crate::vertex_attrs::{GeneralInstanceInput, VertexAttrib};
use renderer_common::geometry_data::MeshVertex;
use std::borrow::Cow;
use wesl::include_wesl;
use wgpu::{Face, RenderPass, ShaderModuleDescriptor, ShaderSource};

pub struct ShadowMapPipeline {
    mesh_pipeline: MeshPipeline,
    render_pipeline: wgpu::RenderPipeline,
}

impl ShadowMapPipeline {
    pub fn new(global_context: &GlobalContext) -> Self {
        let mesh_pipeline = MeshPipeline::new(global_context, false, false, false);
        let mut root_descriptor = mesh_pipeline.prepare(global_context);
        let shadow_map_shader_module =
            global_context
                .device()
                .create_shader_module(ShaderModuleDescriptor {
                    label: Some("shadow_map"),
                    source: ShaderSource::Wgsl(Cow::from(include_wesl!("shadow_map"))),
                });
        root_descriptor.label = Some("shadow_pipeline");
        root_descriptor.vertex = OwnedVertexState {
            module: shadow_map_shader_module,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: vec![MeshVertex::desc(), GeneralInstanceInput::desc()],
        };
        root_descriptor.fragment = None;
        root_descriptor.primitive.cull_mode = Some(Face::Front);
        root_descriptor.multisample.count = 1;
        root_descriptor.depth_stencil.as_mut().unwrap().format = wgpu::TextureFormat::Depth32Float;
        let shadow_pipeline = root_descriptor.to_render_pipeline(global_context.device());
        Self {
            mesh_pipeline,
            render_pipeline: shadow_pipeline,
        }
    }
}

impl RenderPipeline<GeneralInstanceInput> for ShadowMapPipeline {

    fn setup_render(&mut self, render_pass: &mut RenderPass, global_context: &GlobalContext) {
        render_pass.set_pipeline(&self.render_pipeline);
        self.mesh_pipeline.setup_render(render_pass, global_context);
    }

    fn prepare(&self, _global_context: &GlobalContext) -> OwnedRenderPipelineDescriptor<'_> {
        todo!("BLABAB")
    }

    fn is_indirect(&self) -> bool {
        false
    }
}
