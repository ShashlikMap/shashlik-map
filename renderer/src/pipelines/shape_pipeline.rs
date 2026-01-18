use crate::global_context::GlobalContext;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::{OwnedRenderPipelineDescriptor, RenderPipeline};
use crate::vertex_attrs::{ShapeInstanceInput, ShapeVertex, VertexAttrib};
use wgpu::{CompareFunction, RenderPass, include_wgsl};

pub struct ShapePipeline {
    mesh_pipeline: MeshPipeline,
    is_screen: bool,
}

impl ShapePipeline {
    const SHADER_STYLE_GROUP_INDEX: u32 = 1;

    pub fn new(global_context: &GlobalContext, is_screen: bool) -> Self {
        Self {
            mesh_pipeline: MeshPipeline::new(global_context),
            is_screen,
        }
    }
}

impl RenderPipeline for ShapePipeline {
    type InstanceInputType = ShapeInstanceInput;

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &GlobalContext) {
        self.mesh_pipeline.render(render_pass, global_context);
        if let Some(bind_group) = global_context.style_bind_group.as_ref() {
            render_pass.set_bind_group(Self::SHADER_STYLE_GROUP_INDEX, bind_group, &[]);
        }
    }

    fn prepare(&self, global_context: &GlobalContext) -> OwnedRenderPipelineDescriptor<'_> {
        let device = global_context.device();
        let mut mesh_descriptor = self.mesh_pipeline.prepare(global_context);
        let mut stencil = mesh_descriptor.depth_stencil.unwrap();
        stencil.depth_compare = CompareFunction::Always;
        stencil.depth_write_enabled = false;
        mesh_descriptor.depth_stencil = Some(stencil);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shape Render Pipeline Layout"),
            bind_group_layouts: &[
                &self.mesh_pipeline.bind_group_layout,
                &global_context.styles_bind_group_layout,
            ],
            ..Default::default()
        });
        mesh_descriptor.layout = Some(pipeline_layout);

        let shader_module =
            device.create_shader_module(include_wgsl!("../shaders/shape_shader.wgsl"));

        let vertex = &mut mesh_descriptor.vertex;
        if self.is_screen {
            vertex.entry_point = Some("vs_main_screen");
        }
        vertex.module = shader_module.to_owned();
        vertex.buffers = vec![ShapeVertex::desc(), ShapeInstanceInput::desc()];
        let fragment = &mut mesh_descriptor.fragment.as_mut().unwrap();
        fragment.module = shader_module;

        mesh_descriptor.primitive.cull_mode = None;

        mesh_descriptor
    }
}
