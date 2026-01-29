use crate::draw_commands::MeshVertex;
use crate::global_context::GlobalContext;
use crate::mesh::InstanceBuffer;
use crate::mesh::mesh::Mesh;
use crate::mesh_layers::BaseMeshLayer;
use crate::pipelines::RenderPipeline;
use crate::text::glyph_tesselator::GlyphVertexConstructor;
use crate::vertex_attrs::TextInstanceInput;
use cgmath::{Matrix4, SquareMatrix, Vector2};
use lyon::geom::Box2D;
use lyon::geom::euclid::Point2D;
use lyon::lyon_tessellation::{BuffersBuilder, FillOptions, VertexBuffers};
use lyon::path::{Path, Winding};
use lyon::tessellation::FillTessellator;
use wgpu::{Color, RenderPass};

pub struct OrthoMeshLayer<P: RenderPipeline> {
    render_pipeline: P,
    pipeline: Option<wgpu::RenderPipeline>,
    mesh: Mesh,
    instance_buffer: InstanceBuffer<TextInstanceInput>,
}

impl<P: RenderPipeline> OrthoMeshLayer<P> {
    pub fn new(render_pipeline: P, global_context: &mut GlobalContext) -> Self {
        let mut builder = Path::builder();
        builder.add_rectangle(
            &Box2D::new(Point2D::new(0.0, 0.0), Point2D::new(300.0, 300.0)),
            Winding::Positive,
        );
        let path = builder.build();
        let mut geometry: VertexBuffers<MeshVertex, u32> = VertexBuffers::new();
        let vertex_constructor = GlyphVertexConstructor {
            offset: Vector2::new(0.0, 0.0),
            color: Color::BLACK,
        };
        FillTessellator::new()
            .tessellate(
                &path,
                &FillOptions::default(),
                &mut BuffersBuilder::new(&mut geometry, vertex_constructor),
            )
            .unwrap();

        let device = global_context.device();
        let mesh = Mesh::create(device, &geometry);
        Self {
            render_pipeline,
            pipeline: None,
            mesh,
            instance_buffer: InstanceBuffer::default(),
        }
    }
}

impl<P: RenderPipeline> BaseMeshLayer for OrthoMeshLayer<P> {
    fn prepare(&mut self, global_context: &GlobalContext) {
        let descriptor = self.render_pipeline.prepare(global_context);
        self.pipeline = Some(descriptor.to_render_pipeline(global_context.device()));
    }

    fn update(&mut self, global_context: &mut GlobalContext) {
        if self.instance_buffer.buffer.is_none() {
            let screen_size = global_context.view_projection.screen_size;
            let device = global_context.device();
            let queue = global_context.queue();
            let hh = TextInstanceInput {
                position: [
                    screen_size.0 as f32 - 400.0,
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
