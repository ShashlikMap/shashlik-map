use crate::global_context::GlobalContext;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::{OwnedRenderPipelineDescriptor, RenderPipeline};
use crate::vertex_attrs::{ShapeInstanceInput, ShapeVertex, VertexAttrib};
use wgpu::{include_wgsl, BindGroup, BindGroupLayout, CompareFunction, RenderPass};

pub struct ShapePipeline {
    mesh_pipeline: MeshPipeline,
    vs_func_name: Option<&'static str>,
    indirect_buffer_layout: BindGroupLayout,
    indirect: bool

}

impl ShapePipeline {
    const SHADER_STYLE_GROUP_INDEX: u32 = 1;

    pub fn new(global_context: &GlobalContext, vs_func_name: Option<&'static str>, indirect: bool) -> Self {
        let indirect_buffer_layout = global_context.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("shape_indirect_buffer_layout"),
        });
        Self {
            mesh_pipeline: MeshPipeline::new(global_context),
            vs_func_name,
            indirect_buffer_layout,
            indirect
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
        let mut layouts = vec![
            &self.mesh_pipeline.bind_group_layout,
            &global_context.styles_bind_group_layout,
        ];
        if self.indirect {
            layouts.push(&self.indirect_buffer_layout)
        }
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shape Render Pipeline Layout"),
            bind_group_layouts: &layouts,
            ..Default::default()
        });

        let mut mesh_descriptor = self.mesh_pipeline.prepare(global_context);
        let mut stencil = mesh_descriptor.depth_stencil.unwrap();
        stencil.depth_compare = CompareFunction::Always;
        stencil.depth_write_enabled = false;
        mesh_descriptor.depth_stencil = Some(stencil);


        mesh_descriptor.layout = Some(pipeline_layout);

        let shader_module =
            device.create_shader_module(include_wgsl!("../shaders/shape_shader.wgsl"));

        let vertex = &mut mesh_descriptor.vertex;
        vertex.entry_point = self.vs_func_name.or(vertex.entry_point);
        
        vertex.module = shader_module.to_owned();
        vertex.buffers = if self.indirect {
            vec![ShapeVertex::desc()]
        } else {
            vec![ShapeVertex::desc(), ShapeInstanceInput::desc()]
        };
        let fragment = &mut mesh_descriptor.fragment.as_mut().unwrap();
        fragment.module = shader_module;

        mesh_descriptor.primitive.cull_mode = None;

        mesh_descriptor
    }

    fn set_instance_bind_group(&mut self, render_pass: &mut RenderPass, instance_bind_group: &BindGroup) {
        // index 2 in shape pipeline!
        render_pass.set_bind_group(2, instance_bind_group, &[]);
    }

    fn get_instances_layout(&self) -> Option<&BindGroupLayout> {
        if self.indirect {
            Some(&self.indirect_buffer_layout)
        } else {
            None
        }
    }
}
