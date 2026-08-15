use crate::buffer_pool::BufferPool;
use crate::global_context::GlobalContext;
use crate::mesh::InstanceBuffer;
use crate::mesh::mesh::Mesh;
use crate::mesh_buffers::MeshBuffers;
use crate::mesh_layers::{BaseMeshLayer, BaseMeshLayerNew};
use crate::pipelines::{RenderPipeline, WithTexture};
use crate::vertex_attrs::TextInstanceInput;
use glam::Mat4;
use log::error;
use wgpu::{BindGroup, CommandEncoder, RenderPass, TextureFormat, TextureUsages, TextureView};

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
enum TextureType {
    GeneralRgba,
    GeneralRFloat,
    Depth,
}

pub struct OrthoMeshLayer<P: RenderPipeline + WithTexture> {
    render_pipeline: P,
    mesh: Option<Mesh>,
    instance_buffer: InstanceBuffer<TextInstanceInput>,
    mesh_buffers: MeshBuffers,
    texture_bind_group: Option<BindGroup>,
    full_screen_mesh: bool,
    is_bottom_right: bool,
    texture_type: TextureType,
}

impl<P: RenderPipeline + WithTexture> OrthoMeshLayer<P> {
    pub fn new(render_pipeline: P,
               full_screen_mesh: bool,
               is_bottom_right: bool) -> Self {
        Self {
            render_pipeline,
            mesh: None,
            instance_buffer: InstanceBuffer::default(),
            mesh_buffers: MeshBuffers::default(),
            texture_bind_group: None,
            full_screen_mesh,
            is_bottom_right,
            texture_type: TextureType::GeneralRgba,
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
        self.mesh_buffers = MeshBuffers::builder()
            .with_instance_buffer(self.instance_buffer.buffer.clone())
    }
}

impl<P: RenderPipeline + WithTexture> BaseMeshLayer for OrthoMeshLayer<P> {
    fn update(&mut self, _global_context: &mut GlobalContext) {}

    fn compute(&mut self, _encoder: &mut CommandEncoder, _global_context: &mut GlobalContext) {}
    
    fn clear_by_key(&mut self, _key: &str) {}
}

impl<P: RenderPipeline + WithTexture> BaseMeshLayerNew for OrthoMeshLayer<P> {
    fn render_new(&mut self, render_pass: &mut RenderPass, render_pipeline: &mut impl RenderPipeline, global_context: &mut GlobalContext) {
        if let Some(mesh) = self.mesh.as_ref() {
            render_pipeline.setup_render(render_pass, global_context);
            // override params
            // TODO Should it be here or inside pipeline?
            render_pass.set_immediates(
                0,
                bytemuck::bytes_of(&(self.texture_type as u32)),
            );
            if let Some(texture_bind_group) = self.texture_bind_group.as_ref() {
                render_pass.set_bind_group(1, texture_bind_group, &[]);
            }

            render_pipeline.setup_mesh_buffers(render_pass, &self.mesh_buffers);
            let instance_count = self.instance_buffer.length;
            mesh.render_instanced(render_pass, instance_count, false, None);
        }
    }
}