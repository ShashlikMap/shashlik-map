use crate::{GlobalContext, ReceiverExt};
use crate::consts::STYLE_SHADER_PARAMS_COUNT;
use crate::pipelines::mesh_pipeline::MeshPipeline;
use crate::pipelines::{OwnedRenderPipelineDescriptor, RenderPipeline};
use crate::vertex_attrs::{ShapeInstanceInput, ShapeVertex, VertexAttrib};
use tokio::sync::broadcast::Receiver;
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupLayout, CompareFunction, Device, Queue, RenderPass, SurfaceConfiguration,
    include_wgsl,
};

pub struct ShapePipeline {
    mesh_pipeline: MeshPipeline,
    styles_bind_group_layout: BindGroupLayout,
    style_bind_group: Option<BindGroup>,
    style_uniform_rx: Receiver<Vec<[f32; STYLE_SHADER_PARAMS_COUNT]>>,
}

impl ShapePipeline {
    pub fn new(
        device: &Device,
        style_uniform_rx: Receiver<Vec<[f32; STYLE_SHADER_PARAMS_COUNT]>>,
    ) -> Self {
        let styles_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("styles_bind_group_layout"),
            });

        Self {
            mesh_pipeline: MeshPipeline::new(device),
            styles_bind_group_layout,
            style_bind_group: None,
            style_uniform_rx,
        }
    }
}

impl RenderPipeline for ShapePipeline {
    type InstanceInputType = ShapeInstanceInput;

    fn render(
        &mut self,
        render_pass: &mut RenderPass,
        device: &Device,
        queue: &Queue,
        global_context: &mut GlobalContext,
    ) {
        if let Ok(uniforms) = self.style_uniform_rx.no_lagged() {
            // TODO We could reuse the buffer if styles count has not changed
            let styles_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Style Buffer"),
                contents: bytemuck::cast_slice(&uniforms),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

            let styles_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.styles_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: styles_buffer.as_entire_binding(),
                }],
                label: Some("styles_bind_group"),
            });

            self.style_bind_group = Some(styles_bind_group);
        }

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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shape Render Pipeline Layout"),
            bind_group_layouts: &[
                &self.mesh_pipeline.bind_group_layout,
                &self.styles_bind_group_layout,
            ],
            ..Default::default()
        });
        mesh_descriptor.layout = Some(pipeline_layout);

        let shader_module =
            device.create_shader_module(include_wgsl!("../shaders/shape_shader.wgsl"));

        let vertex = &mut mesh_descriptor.vertex;
        vertex.module = shader_module.to_owned();
        vertex.buffers = vec![ShapeVertex::desc(), ShapeInstanceInput::desc()];
        let fragment = &mut mesh_descriptor.fragment.as_mut().unwrap();
        fragment.module = shader_module;

        mesh_descriptor.primitive.cull_mode = None;

        mesh_descriptor
    }
}
