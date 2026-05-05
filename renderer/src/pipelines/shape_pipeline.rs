use crate::global_context::GlobalContext;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::{IndirectInstancesLayout, OwnedRenderPipelineDescriptor, RenderPipeline};
use crate::vertex_attrs::{ShapeInstanceInput, ShapeVertex, VertexAttrib};
use std::borrow::Cow;
use wesl::include_wesl;
use wgpu::{BindGroup, BindGroupLayout, CompareFunction, ComputePass, ComputePipeline, ComputePipelineDescriptor, RenderPass, ShaderModuleDescriptor, ShaderSource, ShaderStages};

pub struct ShapePipeline {
    mesh_pipeline: MeshPipeline,
    vs_func_name: Option<&'static str>,
    indirect_instances_layout: BindGroupLayout,
    indirect_compute_instances_layout: BindGroupLayout,
    indirect_instances_args_layout: BindGroupLayout,
    culling_compute_pipeline: ComputePipeline,
    indirect: bool

}

impl ShapePipeline {
    const SHADER_STYLE_GROUP_INDEX: u32 = 1;

    pub fn new(global_context: &GlobalContext, vs_func_name: Option<&'static str>, indirect: bool) -> Self {
        let indirect_instances_layout = Self::create_indirect_layout(global_context, false);
        let indirect_compute_instances_layout = Self::create_indirect_layout(global_context, true);
        let indirect_instances_args_layout = global_context.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("shape_indirect_args_layout"),
        });

        let mesh_pipeline = MeshPipeline::new(global_context);
        let compute_cull_shader = global_context.device().create_shader_module(ShaderModuleDescriptor {
            label: Some("shape_culling"),
            source: ShaderSource::Wgsl(Cow::from(include_wesl!("shape_culling"))),
        });


        let culling_pipeline_layout = global_context.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shape Compute Pipeline Layout"),
            bind_group_layouts: &[&mesh_pipeline.bind_group_layout,
                &indirect_compute_instances_layout,
                &indirect_instances_args_layout],
            ..Default::default()
        });

        let culling_compute_pipeline = global_context.device().create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("shape_compute_pipeline"),
            layout: Some(&culling_pipeline_layout),
            module: &compute_cull_shader,
            entry_point: None,
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            mesh_pipeline,
            vs_func_name,
            indirect_instances_layout,
            indirect_compute_instances_layout,
            indirect_instances_args_layout,
            culling_compute_pipeline,
            indirect
        }
    }

    fn create_indirect_layout(global_context: &GlobalContext, is_compute_pipeline: bool) -> BindGroupLayout {
        let visibility = if is_compute_pipeline {
            ShaderStages::COMPUTE
        } else {
            ShaderStages::VERTEX
        };
        let label = if is_compute_pipeline {
            "shape_indirect_compute_buffer_layout"
        } else {
            "shape_indirect_buffer_layout"
        };
        global_context.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: !is_compute_pipeline },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: !is_compute_pipeline },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            label: Some(label),
        })
    }
}

impl RenderPipeline for ShapePipeline {
    type InstanceInputType = ShapeInstanceInput;

    fn compute(&mut self, compute_pass: &mut ComputePass, _global_context: &GlobalContext) {
        compute_pass.set_pipeline(&self.culling_compute_pipeline);
        compute_pass.set_bind_group(0, &self.mesh_pipeline.bind_group, &[]);
    }

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
            layouts.push(&self.indirect_instances_layout)
        }
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shape Render Pipeline Layout"),
            bind_group_layouts: &layouts,
            immediate_size: 4,
            ..Default::default()
        });

        let mut mesh_descriptor = self.mesh_pipeline.prepare(global_context);
        let mut stencil = mesh_descriptor.depth_stencil.unwrap();
        stencil.depth_compare = CompareFunction::Always;
        stencil.depth_write_enabled = false;
        mesh_descriptor.depth_stencil = Some(stencil);


        mesh_descriptor.layout = Some(pipeline_layout);


        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("shape_shader"),
            source: ShaderSource::Wgsl(Cow::from(include_wesl!("shape_shader"))),
        });
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

    fn set_instance_bind_group_compute(&mut self, compute_pass: &mut ComputePass, instance_bind_group: &BindGroup, instance_args_bind_group: &BindGroup) {
        compute_pass.set_bind_group(1, instance_bind_group, &[]);
        compute_pass.set_bind_group(2, instance_args_bind_group, &[]);
    }

    fn set_instance_bind_group_render(&mut self, render_pass: &mut RenderPass, instance_bind_group: &BindGroup) {
        // index 2 in shape pipeline for renderer!
        render_pass.set_bind_group(2, instance_bind_group, &[]);
    }

    fn get_instances_layouts(&self) -> Option<IndirectInstancesLayout> {
        if self.indirect {
            Some(IndirectInstancesLayout {
                vertex_layout: &self.indirect_instances_layout,
                compute_layout: &self.indirect_compute_instances_layout,
                common_args_layout: &self.indirect_instances_args_layout,
            })
        } else {
            None
        }
    }

    fn is_indirect(&self) -> bool {
        self.indirect
    }

    fn support_g_buf(&self) -> bool {
        false
    }
}
