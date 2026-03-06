use crate::global_context::GlobalContext;
use crate::mesh::mesh::Mesh;
use crate::mesh::mesh_instance_input::MeshInstanceInput;
use crate::mesh::positioned_mesh::PositionedMesh;
use crate::modifier::render_modifier::SpatialData;
use cgmath::Vector3;
use wgpu::{BindGroup, BindGroupLayout, ColorTargetState, DepthStencilState, Device, Label, MultisampleState, PipelineCompilationOptions, PipelineLayout, PrimitiveState, RenderPass, ShaderModule, TextureView, VertexBufferLayout};

pub mod mesh_pipeline;
pub mod shape_pipeline;
pub mod screen_mesh_pipeline;

pub trait RenderPipeline {
    type InstanceInputType: MeshInstanceInput;

    fn create_positioned_mesh(spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
                              double_style: bool,
                              instance_positions_alpha: Option<Vec<(Vector3<f64>, f32)>>,
                              mesh: Mesh) -> PositionedMesh<Self::InstanceInputType> {
        mesh.to_positioned::<Self::InstanceInputType>(spatial_rx, double_style, instance_positions_alpha)
    }

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &GlobalContext);
    fn prepare(&self, global_context: &GlobalContext) -> OwnedRenderPipelineDescriptor<'_>;
    fn set_instance_bind_group(&mut self, render_pass: &mut RenderPass, instance_bind_group: &BindGroup);

    fn get_instances_layout(&self) -> Option<&BindGroupLayout>;
}

pub trait WithTexture {
    fn create_texture_bind_group(&mut self, texture_view: &TextureView, global_context: &GlobalContext) -> BindGroup;
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
        let owned_fragment_state = descriptor.fragment.as_ref().unwrap();
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
                module: &owned_fragment_state.module,
                entry_point: owned_fragment_state.entry_point,
                targets: &*owned_fragment_state.targets.clone(),
                compilation_options: owned_fragment_state.compilation_options.clone(),
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