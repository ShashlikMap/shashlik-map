use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::{OwnedRenderPipelineDescriptor, RenderPipeline};
use crate::vertex_attrs::{
    ShapeInstanceInput, TextInstanceInput, VertexAttrib, VertexNormal,
};
use crate::GlobalContext;
use wgpu::{
    include_wgsl, CompareFunction, Device, Queue, RenderPass,
    SurfaceConfiguration,
};

pub struct TextPipeline {
    mesh_pipeline: MeshPipeline,
}

impl TextPipeline {
    pub fn new(device: &Device) -> Self {
        Self {
            mesh_pipeline: MeshPipeline::new(device),
        }
    }
}

impl RenderPipeline for TextPipeline {
    type InstanceInputType = ShapeInstanceInput;

    fn render(
        &mut self,
        render_pass: &mut RenderPass,
        device: &Device,
        queue: &Queue,
        global_context: &mut GlobalContext,
    ) {
        // TODO It should be like that
        self.mesh_pipeline
            .render(render_pass, device, queue, global_context);
    }

    fn prepare(
        &self,
        device: &Device,
        config: &SurfaceConfiguration,
    ) -> OwnedRenderPipelineDescriptor<'_> {
        let mut mesh_descriptor = self.mesh_pipeline.prepare(device, config);

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
