use crate::global_context::GlobalContext;
use crate::mesh::InstanceBuffer;
use crate::mesh::mesh::Mesh;
use crate::mesh_layers::BaseMeshLayer;
use crate::pipelines::{RenderPipeline, WithTexture};
use crate::vertex_attrs::TextInstanceInput;
use cgmath::{Matrix4, SquareMatrix};
use log::error;
use wgpu::{BindGroup, CommandEncoder, RenderPass, TextureView};

pub struct OrthoMeshLayer<P: RenderPipeline + WithTexture> {
    render_pipeline: P,
    pipeline: Option<wgpu::RenderPipeline>,
    mesh: Option<Mesh>,
    instance_buffer: InstanceBuffer<TextInstanceInput>,
    texture_bind_group: Option<BindGroup>,
}

impl<P: RenderPipeline + WithTexture> OrthoMeshLayer<P> {
    pub fn new(render_pipeline: P) -> Self {
        Self {
            render_pipeline,
            pipeline: None,
            mesh: None,
            instance_buffer: InstanceBuffer::default(),
            texture_bind_group: None,
        }
    }

    // FIXME Positioning should not be here
    pub fn set_texture(&mut self, texture_view: &TextureView, offset: (f32, f32), global_context: &GlobalContext) {
        let screen_size = global_context.view_projection.screen_size;
        let texture_size = texture_view.texture().size();

        if screen_size.0 == 0.0 || screen_size.1 == 0.0 {
            error!(
                "Not correct screen size for texture positioning {:?}",
                screen_size
            );
            return;
        }
        self.texture_bind_group = Some(
            self.render_pipeline
                .create_texture_bind_group(texture_view, global_context),
        );

        let device = global_context.device();

        if offset.0 == 0.0 {
            self.mesh = Some(Mesh::quad(
                device,
                screen_size.0 as f32,
                screen_size.1 as f32,
            ));
        } else {
            self.mesh = Some(Mesh::quad(
                device,
                texture_size.width as f32,
                texture_size.height as f32,
            ));

        }
        let queue = global_context.queue();
        let position = if offset.0 == 0.0 {
            [
                0.0,
                screen_size.1 as f32,
                0.0,
            ]
        } else {
            [
                screen_size.0 as f32 - texture_size.width as f32 + offset.0,
                screen_size.1 as f32 + offset.0,
                0.0,
            ]
        };
        let attr = TextInstanceInput {
            position,
            color_alpha: 1.0,
            matrix: Matrix4::identity().into(),
            screen_space: 1,
        };
        self.instance_buffer
            .update("quad_instance_buffer", device, queue, &vec![attr]);
    }
}

impl<P: RenderPipeline + WithTexture> BaseMeshLayer for OrthoMeshLayer<P> {
    fn prepare(&mut self, global_context: &GlobalContext) {
        let descriptor = self.render_pipeline.prepare(global_context);
        self.pipeline = Some(descriptor.to_render_pipeline(global_context.device()));
    }

    fn update(&mut self, _global_context: &mut GlobalContext) {}

    fn compute(&mut self, _encoder: &mut CommandEncoder, _global_context: &mut GlobalContext) {}


    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        if global_context.is_g_buffer_render {
            return;
        }
        if let (Some(render_pipeline), Some(mesh)) = (self.pipeline.as_ref(), self.mesh.as_ref()) {
            render_pass.set_pipeline(render_pipeline);

            self.render_pipeline.render(render_pass, global_context);
            if let Some(texture_bind_group) = self.texture_bind_group.as_ref() {
                render_pass.set_bind_group(1, texture_bind_group, &[]);
            }

            mesh.render_instanced(Some(1), render_pass, &self.instance_buffer, false, None);
        }
    }

    fn clear_by_key(&mut self, _key: &str) {}
}
