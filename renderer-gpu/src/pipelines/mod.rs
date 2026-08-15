use crate::global_context::GlobalContext;
use crate::mesh::mesh_instance_input::MeshInstanceInput;
use crate::mesh_buffers::MeshBuffers;
use wgpu::{ColorTargetState, ComputePass, DepthStencilState, Device, Label, MultisampleState, PipelineCompilationOptions, PipelineLayout, PrimitiveState, RenderPass, ShaderModule, VertexBufferLayout};

pub mod mesh_pipeline;
pub mod shape_pipeline;
pub mod screen_mesh_pipeline;
pub mod fill_shadow_map_pipeline;
pub mod g_buf_pipeline;

pub trait RenderPipeline<InstanceInputType: MeshInstanceInput> {
    fn setup_compute(&mut self, _compute_pass: &mut ComputePass, _global_context: &GlobalContext) {}
    fn compute_mesh(&mut self, _compute_pass: &mut ComputePass,
                    _mesh: &MeshBuffers) {}
    fn setup_render(&mut self, render_pass: &mut RenderPass, global_context: &GlobalContext);
    fn setup_mesh_buffers(&mut self, render_pass: &mut RenderPass, mesh_buffers: &MeshBuffers) {
        // by default all instances go to vertex buffer slot 1
        if let Some(buffer) = mesh_buffers.instance_buffer() {
            render_pass.set_vertex_buffer(1, buffer.slice(..));
        }
    }
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
                buffers: &*descriptor.vertex.buffers.into_iter().map(Some).collect::<Vec<_>>(),
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
                cull_mode: descriptor.primitive.cull_mode,
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