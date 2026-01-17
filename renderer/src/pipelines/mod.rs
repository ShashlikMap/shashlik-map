use wgpu::{ColorTargetState, DepthStencilState, Device, Label, MultisampleState, PipelineCompilationOptions, PipelineLayout, PrimitiveState, Queue, RenderPass, ShaderModule, SurfaceConfiguration, VertexBufferLayout};
use crate::GlobalContext;

pub mod mesh_pipeline;

pub trait RenderPipeline {
    fn render(&mut self, render_pass: &mut RenderPass, queue: &Queue, global_context: &mut GlobalContext);
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