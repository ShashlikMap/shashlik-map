use glam::Mat4;
use crate::global_context::{GlobalContext, GlobalRenderStep};
use crate::mesh::InstanceBuffer;
use crate::mesh::mesh::Mesh;
use crate::mesh_layers::BaseMeshLayer;
use crate::pipelines::{RenderPipeline, WithTexture};
use crate::vertex_attrs::TextInstanceInput;
use log::error;
use wgpu::{BindGroup, CommandEncoder, RenderPass, StencilFaceState, TextureFormat, TextureUsages, TextureView};
use crate::buffer_pool::BufferPool;

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
enum TextureType {
    GeneralRgba,
    GeneralRFloat,
    Depth,
}

pub struct OrthoMeshLayer<P: RenderPipeline + WithTexture> {
    render_pipeline: P,
    pipeline: Option<wgpu::RenderPipeline>,
    mesh: Option<Mesh>,
    instance_buffer: InstanceBuffer<TextInstanceInput>,
    texture_bind_group: Option<BindGroup>,
    full_screen_mesh: bool,
    is_bottom_right: bool,
    texture_type: TextureType,
    read_stencil: bool
}

impl<P: RenderPipeline + WithTexture> OrthoMeshLayer<P> {
    pub fn new(render_pipeline: P,
               full_screen_mesh: bool,
               is_bottom_right: bool,
               read_stencil: bool) -> Self {
        Self {
            render_pipeline,
            pipeline: None,
            mesh: None,
            instance_buffer: InstanceBuffer::default(),
            texture_bind_group: None,
            full_screen_mesh,
            is_bottom_right,
            texture_type: TextureType::GeneralRgba,
            read_stencil
        }
    }

    // FIXME Positioning should not be here
    pub fn set_texture(&mut self, texture_view: &TextureView, offset: (f32, f32), global_context: &GlobalContext, buffer_pool: &mut BufferPool) {
        let screen_size = global_context.view_projection.screen_size;

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

        let texture_format = texture_view.texture().format();
        let texture_usage = texture_view.texture().usage();
        self.texture_type = if texture_format.is_depth_stencil_format() {
            TextureType::Depth
        } else if texture_format == TextureFormat::R16Float
            || texture_format == TextureFormat::R32Float {
            TextureType::GeneralRFloat
        } else if texture_format == TextureFormat::Rgba16Float {
            if texture_usage.contains(TextureUsages::STORAGE_BINDING) {
                TextureType::GeneralRFloat
            } else {
                TextureType::GeneralRgba
            }
        } else {
            TextureType::GeneralRgba
        };


        let mesh_size;
        if self.full_screen_mesh {
            mesh_size = (screen_size.0 as f32, screen_size.1 as f32);
            self.mesh = Some(Mesh::quad(
                global_context,
                buffer_pool,
                screen_size.0 as f32,
                screen_size.1 as f32,
            ));
        } else {
            let texture_size = texture_view.texture().size();
            let aspect = texture_size.height as f32 / texture_size.width as f32;
            let width = screen_size.0 as f32 * 0.35;
            let height = aspect * width;
            mesh_size = (width, height);

            self.mesh = Some(Mesh::quad(
                global_context,
                buffer_pool,
                mesh_size.0,
                mesh_size.1
            ));

        }

        let position = [
            if self.is_bottom_right { screen_size.0 as f32 - mesh_size.0 } else { 0.0 } + offset.0,
            screen_size.1 as f32 + offset.1,
            0.0,
        ];
        let attr = TextInstanceInput {
            position,
            color_alpha: 1.0,
            matrix: Mat4::IDENTITY.to_cols_array_2d(),
            screen_space: 1,
        };
        
        self.instance_buffer
            .update("quad_instance_buffer", global_context, &vec![attr]);
    }
}

impl<P: RenderPipeline + WithTexture> BaseMeshLayer for OrthoMeshLayer<P> {
    fn prepare(&mut self, global_context: &GlobalContext) {
        let mut descriptor = self.render_pipeline.prepare(global_context);
        if self.read_stencil {
            descriptor.depth_stencil.as_mut().unwrap().stencil = wgpu::StencilState {
                front: StencilFaceState::IGNORE,
                back: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::NotEqual,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                read_mask: 0xFF,
                write_mask: 0x00,
            };
        }

        self.pipeline = Some(descriptor.to_render_pipeline(global_context.device()));
    }

    fn update(&mut self, _global_context: &mut GlobalContext) {}

    fn compute(&mut self, _encoder: &mut CommandEncoder, _global_context: &mut GlobalContext) {}


    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        if let (Some(render_pipeline), Some(mesh)) = (self.pipeline.as_ref(), self.mesh.as_ref()) {
            render_pass.set_pipeline(render_pipeline);
            if self.read_stencil {
                render_pass.set_stencil_reference(1);
            }

            self.render_pipeline.render(render_pass, global_context);

            // override params
            render_pass.set_immediates(
                0,
                bytemuck::bytes_of(&(self.texture_type as u32)),
            );
            if let Some(texture_bind_group) = self.texture_bind_group.as_ref() {
                render_pass.set_bind_group(1, texture_bind_group, &[]);
            }

            mesh.render_instanced(Some(1), render_pass, false, &self.instance_buffer, false, None);
        }
    }

    fn clear_by_key(&mut self, _key: &str) {}
}
