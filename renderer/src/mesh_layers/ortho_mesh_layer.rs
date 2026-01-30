use crate::draw_commands::MeshVertex;
use crate::global_context::GlobalContext;
use crate::mesh::InstanceBuffer;
use crate::mesh::mesh::Mesh;
use crate::mesh_layers::BaseMeshLayer;
use crate::pipelines::RenderPipeline;
use crate::text::glyph_tesselator::{GlyphVertexConstructor, MeshVertexWithUV};
use crate::vertex_attrs::TextInstanceInput;
use cgmath::{Matrix4, SquareMatrix, Vector2};
use lyon::geom::Box2D;
use lyon::geom::euclid::Point2D;
use lyon::lyon_tessellation::{BuffersBuilder, FillOptions, VertexBuffers};
use lyon::path::{Path, Winding};
use lyon::tessellation::FillTessellator;
use wgpu::{Color, Device, RenderPass};

pub struct OrthoMeshLayer<P: RenderPipeline> {
    render_pipeline: P,
    pipeline: Option<wgpu::RenderPipeline>,
    mesh_size: (f32, f32),
    mesh: Mesh,
    instance_buffer: InstanceBuffer<TextInstanceInput>,
}

impl<P: RenderPipeline> OrthoMeshLayer<P> {
    pub fn new(render_pipeline: P, global_context: &mut GlobalContext) -> Self {
        let mesh_data = Self::create_temp_mesh(global_context.device(), 300.0, 300.0);
        Self {
            render_pipeline,
            pipeline: None,
            mesh_size: mesh_data.0,
            mesh: mesh_data.1,
            instance_buffer: InstanceBuffer::default(),
        }
    }

    fn create_temp_mesh(device: &Device, width: f32, height: f32) -> ((f32,f32), Mesh) {
        let mut builder = Path::builder();
        builder.add_rectangle(
            &Box2D::new(Point2D::new(0.0, 0.0), Point2D::new(width, height)),
            Winding::Positive,
        );
        let path = builder.build();
        let mut geometry: VertexBuffers<MeshVertexWithUV, u32> = VertexBuffers::new();
        let vertex_constructor = GlyphVertexConstructor {
            offset: Vector2::new(0.0, 0.0),
            color: Color::BLACK,
            uv_size: Some((width, height))
        };
        FillTessellator::new()
            .tessellate(
                &path,
                &FillOptions::default(),
                &mut BuffersBuilder::new(&mut geometry, vertex_constructor),
            )
            .unwrap();

        let mesh = Mesh::create(device, &geometry);
        ((width, height), mesh)
    }
}

impl<P: RenderPipeline> BaseMeshLayer for OrthoMeshLayer<P> {
    fn prepare(&mut self, global_context: &GlobalContext) {
        let descriptor = self.render_pipeline.prepare(global_context);
        self.pipeline = Some(descriptor.to_render_pipeline(global_context.device()));
    }

    fn update(&mut self, global_context: &mut GlobalContext) {
        let screen_size = global_context.view_projection.screen_size;

        let nw = (screen_size.0 / 4.0) as f32;
        let nh = (screen_size.1 / 4.0) as f32;
        if self.mesh_size.0 != nw || self.mesh_size.1 != nh {
            let mesh_data = Self::create_temp_mesh(global_context.device(), nw, nh);
            self.mesh_size = mesh_data.0;
            self.mesh = mesh_data.1;
            let device = global_context.device();
            let queue = global_context.queue();
            let hh = TextInstanceInput {
                position: [
                    screen_size.0 as f32 - nw - 100.0,
                    screen_size.1 as f32 - 100.0,
                    0.0,
                ],
                color_alpha: 1.0,
                matrix: Matrix4::identity().into(),
                screen_space: 1,
            };
            self.instance_buffer.update("qqq", device, queue, &vec![hh]);
        }
    }

    fn render(&mut self, render_pass: &mut RenderPass, global_context: &mut GlobalContext) {
        if let Some(render_pipeline) = self.pipeline.as_ref() {
            render_pass.set_pipeline(render_pipeline);

            self.render_pipeline.render(render_pass, global_context);

            self.mesh
                .render_instanced(1, render_pass, &self.instance_buffer);
        }
    }

    fn clear_by_key(&mut self, key: &str) {}
}
