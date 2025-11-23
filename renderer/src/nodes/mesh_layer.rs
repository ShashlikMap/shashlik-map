use crate::nodes::scene_tree::RenderContext;
use crate::nodes::SceneNode;
use crate::pipeline_provider::PipeLineProvider;
use std::rc::Rc;
use wgpu::{CompareFunction, Device, Face, RenderPass, RenderPipeline, ShaderModule, ShaderModuleDescriptor, VertexBufferLayout};
use crate::GlobalContext;

pub struct MeshLayer<'a> {
    shader_module: ShaderModule,
    buffer_layouts: Rc<[VertexBufferLayout<'a>]>,
    custom_cull_mode: Option<Face>,
    pipeline_provider: PipeLineProvider,
    render_pipeline: Option<RenderPipeline>,
    depth_compare: CompareFunction,
}

impl<'a> MeshLayer<'a> {
    pub fn new(
        device: &Device,
        shader_module_desc: ShaderModuleDescriptor<'_>,
        buffer_layouts: Rc<[VertexBufferLayout<'a>]>,
        pipeline_provider: PipeLineProvider,
        custom_cull_mode: Option<Face>,
        depth_compare: CompareFunction,
    ) -> Self {
        let shader_module = device.create_shader_module(shader_module_desc);
        MeshLayer {
            shader_module,
            buffer_layouts,
            custom_cull_mode,
            pipeline_provider,
            render_pipeline: None,
            depth_compare,
        }
    }
}

impl<'a> SceneNode for MeshLayer<'a> {
    fn setup(&mut self, render_context: &mut RenderContext, device: &Device) {
        render_context.depth_compare = self.depth_compare;
        self.render_pipeline = Some(self.pipeline_provider.create(
            device,
            render_context,
            &*self.buffer_layouts,
            &self.shader_module,
            self.custom_cull_mode,
        ));
    }

    fn render(&self, render_pass: &mut RenderPass, _global_context: &mut GlobalContext) {
        render_pass.set_pipeline(self.render_pipeline.as_ref().unwrap());
    }
}
