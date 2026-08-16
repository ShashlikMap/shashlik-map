use crate::buffer_pool::BufferPool;
use crate::global_context::GlobalContext;
use crate::mesh::InstanceBuffer;
use crate::mesh::mesh::Mesh;
use crate::mesh::mesh_instance_input::{MeshInstanceInput};
use crate::mesh_buffers::MeshBuffers;
use crate::mesh_layers::{BaseMeshLayer, LayerAttrMapper, LayerAttrubute, RenderableLayer};
use crate::pipelines::RenderPipeline;
use glam::Mat4;
use log::error;
use wgpu::{RenderPass, TextureView};

pub struct OrthoMeshLayer<I: MeshInstanceInput> {
    attr_map: LayerAttrMapper<I>,
    mesh: Option<Mesh>,
    instance_buffer: InstanceBuffer<I>,
    mesh_buffers: MeshBuffers,
    full_screen_mesh: bool,
    is_bottom_right: bool,
    texture_view: Option<TextureView>,
}

impl<I: MeshInstanceInput> OrthoMeshLayer<I> {
    pub fn new(full_screen_mesh: bool, is_bottom_right: bool, attr_map: LayerAttrMapper<I>) -> Self {
        Self {
            attr_map,
            mesh: None,
            instance_buffer: InstanceBuffer::default(),
            mesh_buffers: MeshBuffers::default(),
            full_screen_mesh,
            is_bottom_right,
            texture_view: None,
        }
    }

    pub fn texture_view(&self) -> Option<&TextureView> {
        self.texture_view.as_ref()
    }

    // FIXME Positioning should not be here
    pub fn set_texture(
        &mut self,
        texture_view: &TextureView,
        offset: (f32, f32),
        global_context: &GlobalContext,
        buffer_pool: &mut BufferPool,
    ) {
        self.texture_view = Some(texture_view.clone());
        let screen_size = global_context.view_projection.screen_size;

        if screen_size.0 == 0.0 || screen_size.1 == 0.0 {
            error!(
                "Not correct screen size for texture positioning {:?}",
                screen_size
            );
            return;
        }

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
                mesh_size.1,
            ));
        }

        let position = [
            if self.is_bottom_right {
                screen_size.0 as f32 - mesh_size.0
            } else {
                0.0
            } + offset.0,
            screen_size.1 as f32 + offset.1,
            0.0,
        ];

        let attr = (self.attr_map)(LayerAttrubute {
            position,
            color_alpha: 1.0,
            matrix: Mat4::IDENTITY.to_cols_array_2d(),
            screen_space: 1,
            ..Default::default()
        });
       
        self.instance_buffer
            .update("quad_instance_buffer", global_context, &vec![attr]);
        self.mesh_buffers =
            MeshBuffers::builder().with_instance_buffer(self.instance_buffer.buffer.clone())
    }
}

impl <I: MeshInstanceInput> BaseMeshLayer for OrthoMeshLayer<I> {
    fn update(&mut self, _global_context: &mut GlobalContext) {}

    fn clear_by_key(&mut self, _key: &str) {}
}

impl<I: MeshInstanceInput> RenderableLayer<I> for OrthoMeshLayer<I> {
    fn render(
        &mut self,
        render_pass: &mut RenderPass,
        render_pipeline: &mut impl RenderPipeline<I>,
        global_context: &mut GlobalContext,
    ) {
        if let Some(mesh) = self.mesh.as_ref() {
            render_pipeline.setup_render(render_pass, global_context);

            render_pipeline.setup_mesh_buffers(render_pass, &self.mesh_buffers);
            let instance_count = self.instance_buffer.length;
            mesh.render_instanced(render_pass, instance_count, false, None);
        }
    }
}
