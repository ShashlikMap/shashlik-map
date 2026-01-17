use crate::mesh::mesh::Mesh;
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::mesh_node::{MeshInstanceInput, PositionedMesh};
use crate::GlobalContext;
use cgmath::Vector3;
use wgpu::{ColorTargetState, DepthStencilState, Device, Label, MultisampleState, PipelineCompilationOptions, PipelineLayout, PrimitiveState, Queue, RenderPass, ShaderModule, SurfaceConfiguration, VertexBufferLayout};

pub mod mesh_pipeline;
pub mod shape_pipeline;
pub mod text_pipeline;

pub trait RenderPipeline {
    type InstanceInputType: MeshInstanceInput;

    fn create_positioned_mesh(device: &Device,
                              instance_positions: Option<Vec<Vector3<f64>>>,
                              spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
                              is_two_instances: bool,
                              with_collisions: bool,
                              mesh: Mesh) -> PositionedMesh<Self::InstanceInputType> {
        mesh.to_positioned_with_instances::<Self::InstanceInputType>(device, instance_positions, spatial_rx, is_two_instances, with_collisions)
    }

    fn render(&mut self, render_pass: &mut RenderPass, device: &Device, queue: &Queue, global_context: &mut GlobalContext);
    fn prepare(&self, device: &Device, config: &SurfaceConfiguration) -> OwnedRenderPipelineDescriptor<'_>;
}

#[derive(Clone, Debug)]
pub struct OwnedRenderPipelineDescriptor<'a> {
    pub label: Label<'a>,
    pub layout: Option<PipelineLayout>,
    pub vertex: OwnedVertexState<'a>,
    pub primitive: PrimitiveState,
    pub depth_stencil: Option<DepthStencilState>,
    pub multisample: MultisampleState,
    pub fragment: Option<OwnedFragmentState<'a>>,
}

impl OwnedRenderPipelineDescriptor<'_> {
    pub fn to_render_pipeline(self, device: &Device) -> wgpu::RenderPipeline {
        let descriptor = self;
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: descriptor.layout.as_ref(),
            vertex: wgpu::VertexState {
                module: &descriptor.vertex.module,
                entry_point: descriptor.vertex.entry_point,
                buffers: &*descriptor.vertex.buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &descriptor.vertex.module,
                entry_point: Some("fs_main"),
                targets: &*descriptor.fragment.unwrap().targets,
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: descriptor.primitive.cull_mode,
                polygon_mode: wgpu::PolygonMode::Fill,
                ..Default::default()
            },
            depth_stencil: descriptor.depth_stencil,
            multisample: descriptor.multisample,
            // Useful for optimizing shader compilation on Android
            cache: None,
            multiview_mask: None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct OwnedVertexState<'a> {
    pub module: ShaderModule,
    pub entry_point: Option<&'a str>,
    pub compilation_options: PipelineCompilationOptions<'a>,
    pub buffers: Vec<VertexBufferLayout<'a>>,
}

#[derive(Clone, Debug)]
pub struct OwnedFragmentState<'a> {
    pub module: ShaderModule,
    pub entry_point: Option<&'a str>,
    pub compilation_options: PipelineCompilationOptions<'a>,
    pub targets: Vec<Option<ColorTargetState>>,
}