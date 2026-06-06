use crate::draw_commands::MeshVertex;
use crate::vertex_attrs::MeshVertexWithUV;
use glam::Vec2;
use lyon::lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor, LineCap, LineJoin, StrokeVertexConstructor, VertexBuffers};
use lyon::path::{Builder, Path};
use lyon::tessellation::{StrokeOptions, StrokeTessellator};
use rustybuzz::ttf_parser::OutlineBuilder;
use std::mem;
use wgpu::Color;
#[derive(Clone)]
pub struct GlyphTesselator {
    builder: Builder,
    scale: f32,
}

impl GlyphTesselator {
    pub(crate) fn create_path(&mut self) -> Path {
        mem::replace(&mut self.builder, Builder::new()).build()
    }
    pub(crate) fn tessellate_fill(
        &self,
        buffer: &mut VertexBuffers<MeshVertexWithUV, u32>,
        path: &Path,
        color: Color,
    ) {
        let vertex_constructor = GlyphVertexConstructor { offset: Vec2::new(0.0, 0.0), color };
        let mut tessellator = FillTessellator::new();
        if !tessellator
            .tessellate(
                path,
                &FillOptions::default().with_fill_rule(lyon::path::FillRule::NonZero),
                &mut BuffersBuilder::new(buffer, vertex_constructor),
            )
            .is_ok()
        {
            panic!("Glyph fill tessellate failed.");
        }
    }

    pub(crate) fn tessellate_stroke(
        &self,
        buffer: &mut VertexBuffers<MeshVertexWithUV, u32>,
        path: &Path,
        size: f32,
        color: Color,
    ) {
        let vertex_constructor = GlyphVertexConstructor { offset: Vec2::new(0.0, 0.0), color };
        let mut tessellator = StrokeTessellator::new();
        if !tessellator
            .tessellate(
                path,
                &StrokeOptions::default()
                    .with_line_join(LineJoin::Round)
                    .with_line_cap(LineCap::Round)
                    .with_line_width(size),
                &mut BuffersBuilder::new(buffer, vertex_constructor),
            )
            .is_ok()
        {
            panic!("GLyph stroke tessellate failed.");
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
