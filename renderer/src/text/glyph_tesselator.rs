use glam::Vec2;
use crate::draw_commands::MeshVertex;
use crate::vertex_attrs::MeshVertexWithUV;
use lyon::lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor, LineCap, StrokeVertexConstructor, VertexBuffers};
use lyon::path::{Builder, LineJoin, Path};
use lyon::tessellation::{StrokeOptions, StrokeTessellator};
use rustybuzz::ttf_parser::OutlineBuilder;
use wgpu::Color;
#[derive(Clone)]
pub struct GlyphTesselator {
    builder: Builder,
    scale: f32,
}

impl GlyphTesselator {
    pub(crate) fn tessellate_fill(
        self,
        offset: Vec2,
        color: Color,
    ) -> VertexBuffers<MeshVertexWithUV, u32> {
        let mut buffer = VertexBuffers::new();
        let vertex_constructor = GlyphVertexConstructor { offset, color };
        let mut tessellator = FillTessellator::new();
        if tessellator
            .tessellate(
                &self.builder.build(),
                &FillOptions::default().with_fill_rule(lyon::path::FillRule::NonZero),
                &mut BuffersBuilder::new(&mut buffer, vertex_constructor),
            )
            .is_ok()
        {
            buffer
        } else {
            panic!("Tessellate failed.");
        }
    }

    pub(crate) fn tessellate_stroke(
        self,
        offset: Vec2,
        color: Color,
    ) -> VertexBuffers<MeshVertexWithUV, u32> {
        let mut buffer = VertexBuffers::new();
        let vertex_constructor = GlyphVertexConstructor { offset, color };
        let mut tessellator = StrokeTessellator::new();
        if tessellator
            .tessellate(
                &self.builder.build(),
                &StrokeOptions::default()
                    .with_line_cap(LineCap::Round)
                    .with_line_join(LineJoin::Round).with_line_width(12.0),
                &mut BuffersBuilder::new(&mut buffer, vertex_constructor),
            )
            .is_ok()
        {
            buffer
        } else {
            panic!("Tessellate failed.");
        }
    }
}

impl GlyphTesselator {
    pub fn new(scale: f32) -> Self {
        Self {
            scale,
            builder: Path::builder(),
        }
    }
}

impl OutlineBuilder for GlyphTesselator {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder
            .begin(lyon::geom::point(x * self.scale, y * self.scale));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder
            .line_to(lyon::geom::point(x * self.scale, y * self.scale));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.builder.quadratic_bezier_to(
            lyon::geom::point(x1 * self.scale, y1 * self.scale),
            lyon::geom::point(x * self.scale, y * self.scale),
        );
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.builder.cubic_bezier_to(
            lyon::geom::point(x1 * self.scale, y1 * self.scale),
            lyon::geom::point(x2 * self.scale, y2 * self.scale),
            lyon::geom::point(x * self.scale, y * self.scale),
        );
    }

    fn close(&mut self) {
        self.builder.end(true);
    }
}

struct GlyphVertexConstructor {
    offset: Vec2,
    color: Color,
}

impl FillVertexConstructor<MeshVertexWithUV> for GlyphVertexConstructor {
    fn new_vertex(&mut self, vertex: FillVertex) -> MeshVertexWithUV {
        MeshVertexWithUV {
            mesh_vertex: MeshVertex {
                position: [
                    vertex.position().x + self.offset.x,
                    vertex.position().y + self.offset.y,
                    0.0,
                ],
                normals: [0.0, 0.0, 0.0],
            },
            color: [self.color.r as f32, self.color.g as f32, self.color.b as f32],
            uv: [0.0, 0.0],
        }
    }
}

impl StrokeVertexConstructor<MeshVertexWithUV> for GlyphVertexConstructor {
    fn new_vertex(&mut self, vertex: lyon::tessellation::StrokeVertex) -> MeshVertexWithUV {
        MeshVertexWithUV {
            mesh_vertex: MeshVertex {
                position: [
                    vertex.position().x + self.offset.x,
                    vertex.position().y + self.offset.y,
                    0.0,
                ],
                normals: [0.0, 0.0, 0.0],
            },
            color: [self.color.r as f32, self.color.g as f32, self.color.b as f32],
            uv: [0.0, 0.0],
        }
    }
}
