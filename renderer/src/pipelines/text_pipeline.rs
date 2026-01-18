use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::{OwnedRenderPipelineDescriptor, RenderPipeline};
use crate::vertex_attrs::{ShapeInstanceInput, TextInstanceInput, VertexAttrib, VertexNormal};
use crate::GlobalContext;
use wgpu::{include_wgsl, CompareFunction, RenderPass};

pub struct TextPipeline {
    mesh_pipeline: MeshPipeline,
}

impl TextPipeline {
    pub fn new(global_context: &GlobalContext) -> Self {
        Self {
            mesh_pipeline: MeshPipeline::new(global_context),
        }
    }
}

impl RenderPipeline for TextPipeline {
    type InstanceInputType = ShapeInstanceInput;

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &GlobalContext) {
        self.mesh_pipeline.render(render_pass, global_context);
    }

    fn prepare(&self, global_context: &GlobalContext) -> OwnedRenderPipelineDescriptor<'_> {
        let mut mesh_descriptor = self.mesh_pipeline.prepare(global_context);

        let device = global_context.device();

        let mut stencil = mesh_descriptor.depth_stencil.unwrap();
        stencil.depth_compare = CompareFunction::Always;
        stencil.depth_write_enabled = false;
        mesh_descriptor.depth_stencil = Some(stencil);

        let shader_module =
            device.create_shader_module(include_wgsl!("../shaders/text_shader.wgsl"));

        let vertex = &mut mesh_descriptor.vertex;
        vertex.module = shader_module.to_owned();
        vertex.buffers = vec![VertexNormal::desc(), TextInstanceInput::desc()];
        let fragment = &mut mesh_descriptor.fragment.as_mut().unwrap();
        fragment.module = shader_module;

        mesh_descriptor.primitive.cull_mode = None;

        mesh_descriptor
    }
}
