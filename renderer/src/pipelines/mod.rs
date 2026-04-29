use glam::DVec3;
use crate::global_context::GlobalContext;
use crate::mesh::mesh::Mesh;
use crate::mesh::mesh_instance_input::MeshInstanceInput;
use crate::mesh::positioned_mesh::PositionedMesh;
use crate::modifier::render_modifier::SpatialData;
use wgpu::{BindGroup, BindGroupLayout, ColorTargetState, ComputePass, DepthStencilState, Device, Label, MultisampleState, PipelineCompilationOptions, PipelineLayout, PrimitiveState, RenderPass, ShaderModule, TextureView, VertexBufferLayout};

pub mod mesh_pipeline;
pub mod shape_pipeline;
pub mod screen_mesh_pipeline;

pub trait RenderPipeline {
    type InstanceInputType: MeshInstanceInput;

    fn create_positioned_mesh(spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
                              double_style: bool,
                              instance_positions_alpha: Option<Vec<(DVec3, f32)>>,
                              mesh: Mesh) -> PositionedMesh<Self::InstanceInputType> {
        mesh.to_positioned::<Self::InstanceInputType>(spatial_rx, double_style, instance_positions_alpha)
    }

    fn compute(&mut self, compute_pass: &mut ComputePass, global_context: &GlobalContext);
    fn render(&mut self, render_pass: &mut RenderPass, global_context: &GlobalContext);
    fn prepare(&self, global_context: &GlobalContext) -> OwnedRenderPipelineDescriptor<'_>;
    fn set_instance_bind_group_compute(&mut self, compute_pass: &mut ComputePass, instance_bind_group: &BindGroup, instance_args_bind_group: &BindGroup);
    fn set_instance_bind_group_render(&mut self, render_pass: &mut RenderPass, instance_bind_group: &BindGroup);

    fn get_instances_layouts(&self) -> Option<(&BindGroupLayout, &BindGroupLayout)>;

    fn is_indirect(&self) -> bool;
    fn support_g_buf(&self) -> bool;
}

pub trait WithTexture {
    fn create_texture_bind_group(&mut self, texture_view: &TextureView, global_context: &GlobalContext) -> BindGroup;
}

pub trait WithSSAOTexture {
    fn update_ssao_texture(&mut self, texture_view: &TextureView, global_context: &GlobalContext);
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
            label: descriptor.label,
            layout: descriptor.layout.as_ref(),
            vertex: wgpu::VertexState {
                module: &descriptor.vertex.module,
                entry_point: descriptor.vertex.entry_point,
                buffers: &*descriptor.vertex.buffers,
                compilation_options: Default::default(),
            },
            fragment: descriptor.fragment.as_ref().map(|owned_fragment_state| {
                wgpu::FragmentState {
                    module: &owned_fragment_state.module,
                    entry_point: owned_fragment_state.entry_point,
                    targets: &*owned_fragment_state.targets,
                    compilation_options: owned_fragment_state.compilation_options.clone(),
                }
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