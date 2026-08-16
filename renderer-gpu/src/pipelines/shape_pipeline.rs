use crate::global_context::GlobalContext;
use crate::mesh_buffers::MeshBuffers;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::{OwnedRenderPipelineDescriptor, RenderPipeline};
use crate::vertex_attrs::{ShapeInstanceInput, ShapeVertex, VertexAttrib};
use log::error;
use std::borrow::Cow;
use wesl::include_wesl;
use wgpu::{BindGroup, BindGroupLayout, Buffer, CompareFunction, ComputePass, ComputePipeline, ComputePipelineDescriptor, Device, RenderPass, ShaderModuleDescriptor, ShaderSource, ShaderStages};
use renderer_common::WorldShapeFeatureLayerTag;
use crate::bind_group_cache::{BindGroupCache, BindGroupKey};

pub struct ShapePipeline {
    mesh_pipeline: MeshPipeline,
    pipeline: Option<wgpu::RenderPipeline>,
    vs_func_name: Option<&'static str>,
    bind_group_cache: BindGroupCache,
    indirect_render_instances_layout: BindGroupLayout,
    indirect_compute_instances_layout: BindGroupLayout,
    indirect_compute_instances_args_layout: BindGroupLayout,
    culling_compute_pipeline: ComputePipeline,
    reset_culling_compute_pipeline: ComputePipeline,
    indirect: bool,
    single_instance_step: bool,
}

impl ShapePipeline {
    const SHADER_STYLE_GROUP_INDEX: u32 = 1;

    const COMPUTE_LAYOUT_ID: usize = 0;
    const RENDER_LAYOUT_ID: usize = 1;

    pub fn from_world_shape_tags(global_context: &GlobalContext,
                                 world_shape_feature_layer_tag: Vec<WorldShapeFeatureLayerTag>) -> Vec<(String, Self)> {
        world_shape_feature_layer_tag
            .into_iter()
            .map(|tag| {
                let pipeline = ShapePipeline::new(
                    global_context,
                    tag.vertex_shader,
                    tag.indirect,
                    tag.single_instance_step,
                );
                (tag.name.to_string(), pipeline)
            })
            .collect()
    }

    pub fn new(global_context: &GlobalContext,
               vs_func_name: Option<&'static str>,
               indirect: bool,
               single_instance_step: bool) -> Self {
        let indirect_render_instances_layout = Self::create_indirect_layout(global_context, false);
        let indirect_compute_instances_layout = Self::create_indirect_layout(global_context, true);
        let indirect_compute_instances_args_layout = global_context.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let mesh_pipeline = MeshPipeline::new(global_context, false, false, false);
        let compute_cull_shader = global_context.device().create_shader_module(ShaderModuleDescriptor {
            label: Some("shape_culling"),
            source: ShaderSource::Wgsl(Cow::from(include_wesl!("shape_culling"))),
        });


        let culling_pipeline_layout = global_context.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shape Compute Pipeline Layout"),
            bind_group_layouts: &[Some(&mesh_pipeline.bind_group_layout),
                Some(&indirect_compute_instances_layout),
                Some(&indirect_compute_instances_args_layout)],
            ..Default::default()
        });

        let reset_culling_pipeline_layout = global_context.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shape Compute Reset Pipeline Layout"),
            bind_group_layouts: &[None,
                Some(&indirect_compute_instances_layout),
                Some(&indirect_compute_instances_args_layout)],
            ..Default::default()
        });

        let culling_compute_pipeline = global_context.device().create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("shape_compute_pipeline"),
            layout: Some(&culling_pipeline_layout),
            module: &compute_cull_shader,
            entry_point: Some("compute_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let reset_culling_compute_pipeline = global_context.device().create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("shape_reset_compute_pipeline"),
            layout: Some(&reset_culling_pipeline_layout),
            module: &compute_cull_shader,
            entry_point: Some("compute_reset_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let mut result = Self {
            mesh_pipeline,
            pipeline: None,
            vs_func_name,
            bind_group_cache: BindGroupCache::new(global_context.device()),
            indirect_render_instances_layout,
            indirect_compute_instances_layout,
            indirect_compute_instances_args_layout,
            culling_compute_pipeline,
            reset_culling_compute_pipeline,
            indirect,
            single_instance_step,
        };
        let descriptor = result.prepare(global_context);
        result.pipeline = Some(descriptor.to_render_pipeline(global_context.device()));

        result
    }

    fn prepare(&self, global_context: &GlobalContext) -> OwnedRenderPipelineDescriptor<'_> {
        let device = global_context.device();
        let mut layouts = vec![
            Some(&self.mesh_pipeline.bind_group_layout),
            Some(&global_context.styles_bind_group_layout),
        ];
        if self.indirect {
            layouts.push(Some(&self.indirect_render_instances_layout))
        }
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shape Render Pipeline Layout"),
            bind_group_layouts: &layouts,
            immediate_size: 4,
            ..Default::default()
        });

        let mut mesh_descriptor = self.mesh_pipeline.prepare(global_context);
        mesh_descriptor.label = Some("Shape Pipeline");
        let mut stencil = mesh_descriptor.depth_stencil.unwrap();
        stencil.depth_compare = Some(CompareFunction::Always);
        stencil.depth_write_enabled = Some(false);
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
            let layout = if self.single_instance_step {
                ShapeInstanceInput::desc_no_stride()
            } else {
                ShapeInstanceInput::desc()
            };
            vec![ShapeVertex::desc(), layout]
        };
        let fragment = &mut mesh_descriptor.fragment.as_mut().unwrap();
        fragment.module = shader_module;

        mesh_descriptor.primitive.cull_mode = None;

        mesh_descriptor
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

    fn create_instance_bind_group(device: &Device,
                                  bind_group_layout: &BindGroupLayout,
                                  instance_buffer: &Buffer,
                                  culled_buffer: &Buffer,
                                  label: &'static str) -> BindGroup {
        device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: instance_buffer
                        .as_entire_binding(),
                },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: culled_buffer.as_entire_binding(),
                    }],
                label: Some(label),
            })
    }
}

impl RenderPipeline<ShapeInstanceInput> for ShapePipeline {

    fn setup_compute(&mut self, _compute_pass: &mut ComputePass, _global_context: &GlobalContext) {
        // we call it every frame to clear internal bind group cache for indirect buffers
        if self.indirect {
            self.bind_group_cache.clear_if_needed();
        }
    }

    fn compute_mesh(&mut self,
                    compute_pass: &mut ComputePass,
                    mesh_buffers: &MeshBuffers<ShapeInstanceInput>) {
        if self.indirect {
            if let Some(instance_args_buffer) = mesh_buffers.args_buffer_with_id() &&
                let Some(culled_buffer) = mesh_buffers.culled_buffer_with_id() &&
                let Some(instance_buffer) = mesh_buffers.instance_buffer_with_id() {
                {
                    let instance_bind_group = self.bind_group_cache.get_bind_group_or_create(
                        BindGroupKey::new(Self::COMPUTE_LAYOUT_ID, &[instance_buffer.id(), culled_buffer.id()]), |device| {
                            Self::create_instance_bind_group(device,
                                                            &self.indirect_compute_instances_layout,
                                                            instance_buffer.buffer(),
                                                            culled_buffer.buffer(), "Indirect compute BindGroup")
                        });
                    compute_pass.set_bind_group(1, instance_bind_group, &[]);
                }

                {
                    let instances_args_bind_group = self.bind_group_cache.get_bind_group_or_create(
                        BindGroupKey::new(Self::COMPUTE_LAYOUT_ID, &[instance_args_buffer.id()]),
                        |device| {
                            device
                                .create_bind_group(&wgpu::BindGroupDescriptor {
                                    layout: &self.indirect_compute_instances_args_layout,
                                    entries: &[wgpu::BindGroupEntry {
                                        binding: 0,
                                        resource: instance_args_buffer.buffer().as_entire_binding(),
                                    }],
                                    label: Some("instances_bind_args_group"),
                                })
                        },
                    );
                    compute_pass.set_bind_group(2, instances_args_bind_group, &[]);
                }

                compute_pass.set_pipeline(&self.reset_culling_compute_pipeline);
                compute_pass.dispatch_workgroups(1, 1, 1);

                compute_pass.set_pipeline(&self.culling_compute_pipeline);
                compute_pass.set_bind_group(0, &self.mesh_pipeline.bind_group, &[]);
            }
        }
    }

    fn setup_render(&mut self, render_pass: &mut RenderPass, global_context: &GlobalContext) {
        if let Some(pipeline) = self.pipeline.as_ref() {
            render_pass.set_pipeline(pipeline);
        }

        self.mesh_pipeline.setup_render(render_pass, global_context);
        if let Some(bind_group) = global_context.style_bind_group.as_ref() {
            render_pass.set_bind_group(Self::SHADER_STYLE_GROUP_INDEX, bind_group, &[]);
        }
    }

    fn setup_mesh_buffers(&mut self, render_pass: &mut RenderPass, mesh_buffers: &MeshBuffers<ShapeInstanceInput>) {
        if self.indirect && let Some(instance_buffer) = mesh_buffers.instance_buffer_with_id()
            && let Some(culled_buffer) = mesh_buffers.culled_buffer_with_id() {
            let instance_bind_group = self.bind_group_cache.get_bind_group_or_create(
                BindGroupKey::new(Self::RENDER_LAYOUT_ID, &[instance_buffer.id(), culled_buffer.id()]), |device| {
                    Self::create_instance_bind_group(device,
                                                    &self.indirect_render_instances_layout,
                                                    instance_buffer.buffer(),
                                                    culled_buffer.buffer(), "Indirect render BindGroup")
                });
            render_pass.set_bind_group(2, instance_bind_group, &[]);
        } else if let Some(buffer) = mesh_buffers.instance_buffer() {
            if buffer.size() > 0 {
                render_pass.set_vertex_buffer(1, buffer.slice(..));
            } else {
                error!("Buffer is empty!");
            }
        }
    }
}
