use crate::vertex_attrs::ShapeVertex;
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertexConstructor, StrokeOptions,
    StrokeTessellator, StrokeVertex, StrokeVertexConstructor, VertexBuffers,
};
use lyon::math::{Point, Vector};
use lyon::path::PathEvent;
use lyon::tessellation;
use usvg::{tiny_skia_path, Group, Size, Transform};
// Taken from here https://github.com/nical/lyon/blob/main/examples/wgpu_svg/src/main.rs
// with some minor changes

pub fn svg_parse(
    icon: &[u8],
    width: f32,
    style_index: usize,
) -> VertexBuffers<ShapeVertex, u32> {
    let mut fill_tess = FillTessellator::new();
    let mut stroke_tess = StrokeTessellator::new();
    let mut mesh: VertexBuffers<ShapeVertex, _> = VertexBuffers::new();

    let opt = usvg::Options::default();
    let db = usvg::fontdb::Database::new();
    let rtree = usvg::Tree::from_data(&icon, &opt, &db).unwrap();
    let bbox = rtree.view_box().rect;
    let original_size = Size::from_wh(bbox.width(), bbox.height()).unwrap();
    // let size = Size(width, height);
    let scale = width / original_size.width();
    let mut transforms = Vec::new();
    let mut primitives = Vec::new();

    let mut prev_transform = usvg::Transform {
        sx: f32::NAN,
        kx: f32::NAN,
        ky: f32::NAN,
        sy: f32::NAN,
        tx: f32::NAN,
        ty: f32::NAN,
    };
    collect_geom(
        &rtree.root(),
        &mut prev_transform,
        &mut transforms,
        &mut primitives,
        &mut fill_tess,
        &mut mesh,
        &mut stroke_tess,
        original_size,
        style_index as u32,
        scale,
    );

    mesh
}

fn collect_geom(
    group: &Group,
    prev_transform: &mut Transform,
    transforms: &mut Vec<GpuTransform>,
    primitives: &mut Vec<GpuPrimitive>,
    fill_tess: &mut FillTessellator,
    mesh: &mut VertexBuffers<ShapeVertex, u32>,
    stroke_tess: &mut StrokeTessellator,
    original_size: Size,
    style_index: u32,
    scale: f32,
) {
    for node in group.children() {
        if let usvg::Node::Group(group) = node {
            collect_geom(
                group,
                prev_transform,
                transforms,
                primitives,
                fill_tess,
                mesh,
                stroke_tess,
                original_size,
                style_index,
                scale,
            )
        } else if let usvg::Node::Path(p) = &node {
            let t = node.abs_transform();
            if t != *prev_transform {
                transforms.push(GpuTransform {
                    data0: [t.sx, t.kx, t.ky, t.sy],
                    data1: [t.tx, t.ty, 0.0, 0.0],
                });
            }
            *prev_transform = t;

            let transform_idx = transforms.len() as u32 - 1;

            if let Some(fill) = p.fill() {
                // fall back to always use color fill
                // no gradients (yet?)
                let color = match fill.paint() {
                    usvg::Paint::Color(c) => *c,
                    _ => FALLBACK_COLOR,
                };

                primitives.push(GpuPrimitive::new(
                    transform_idx,
                    color,
                    fill.opacity().get(),
                ));

                fill_tess
                    .tessellate(
                        convert_path(p),
                        &FillOptions::tolerance(0.01),
                        &mut BuffersBuilder::new(
                            mesh,
                            VertexCtor {
                                original_size,
                                scale,
                                style_index,
                            },
                        ),
                    )
                    .expect("Error during tessellation!");
            }

            if let Some(stroke) = p.stroke() {
                let (stroke_color, stroke_opts) = convert_stroke(stroke);
                primitives.push(GpuPrimitive::new(
                    transform_idx,
                    stroke_color,
                    stroke.opacity().get(),
                ));
                let _ = stroke_tess.tessellate(
                    convert_path(p),
                    &stroke_opts.with_tolerance(0.01),
                    &mut BuffersBuilder::new(
                        mesh,
                        VertexCtor {
                            original_size,
                            scale,
                            style_index,
                        },
                    ),
                );
            }
        }
    }
}

const FALLBACK_COLOR: usvg::Color = usvg::Color {
    red: 5,
    green: 5,
    blue: 5,
};

fn convert_stroke(s: &usvg::Stroke) -> (usvg::Color, StrokeOptions) {
    let color = match s.paint() {
        usvg::Paint::Color(c) => *c,
        _ => FALLBACK_COLOR,
    };
    let linecap = match s.linecap() {
        usvg::LineCap::Butt => tessellation::LineCap::Butt,
        usvg::LineCap::Square => tessellation::LineCap::Square,
        usvg::LineCap::Round => tessellation::LineCap::Round,
    };
    let linejoin = match s.linejoin() {
        usvg::LineJoin::Miter => tessellation::LineJoin::Miter,
        usvg::LineJoin::MiterClip => tessellation::LineJoin::MiterClip,
        usvg::LineJoin::Bevel => tessellation::LineJoin::Bevel,
        usvg::LineJoin::Round => tessellation::LineJoin::Round,
    };

    let opt = StrokeOptions::tolerance(0.01)
        .with_line_width(s.width().get())
        .with_line_cap(linecap)
        .with_line_join(linejoin);

    (color, opt)
}

fn convert_path(p: &'_ usvg::Path) -> PathConvIter<'_> {
    PathConvIter {
        iter: p.data().segments(),
        first: Point::new(0.0, 0.0),
        prev: Point::new(0.0, 0.0),
        deferred: None,
        needs_end: false,
    }
}

/// Some glue between usvg's iterators and lyon's.
struct PathConvIter<'a> {
    iter: tiny_skia_path::PathSegmentsIter<'a>,
    prev: Point,
    first: Point,
    needs_end: bool,
    deferred: Option<PathEvent>,
}

impl<'l> Iterator for PathConvIter<'l> {
    type Item = PathEvent;
    fn next(&mut self) -> Option<PathEvent> {
        if self.deferred.is_some() {
            return self.deferred.take();
        }

        let next = self.iter.next();
        match next {
            Some(tiny_skia_path::PathSegment::MoveTo(pt)) => {
                if self.needs_end {
                    let last = self.prev;
                    let first = self.first;
                    self.needs_end = false;
                    self.prev = Point::new(pt.x, pt.y);
                    self.deferred = Some(PathEvent::Begin { at: self.prev });
                    self.first = self.prev;
                    Some(PathEvent::End {
                        last,
                        first,
                        close: false,
                    })
                } else {
                    self.first = Point::new(pt.x, pt.y);
                    self.needs_end = true;
                    Some(PathEvent::Begin { at: self.first })
                }
            }
            Some(tiny_skia_path::PathSegment::LineTo(pt)) => {
                self.needs_end = true;
                let from = self.prev;
                self.prev = Point::new(pt.x, pt.y);
                Some(PathEvent::Line {
                    from,
                    to: self.prev,
                })
            }
            Some(tiny_skia_path::PathSegment::CubicTo(p1, p2, p0)) => {
                self.needs_end = true;
                let from = self.prev;
                self.prev = Point::new(p0.x, p0.y);
                Some(PathEvent::Cubic {
                    from,
                    ctrl1: Point::new(p1.x, p1.y),
                    ctrl2: Point::new(p2.x, p2.y),
                    to: self.prev,
                })
            }
            Some(tiny_skia_path::PathSegment::QuadTo(p0, p1)) => {
                self.needs_end = true;
                let from = self.prev;
                self.prev = Point::new(p1.x, p1.y);
                Some(PathEvent::Quadratic {
                    from,
                    ctrl: Point::new(p0.x, p0.y),
                    to: self.prev,
                })
            }
            Some(tiny_skia_path::PathSegment::Close) => {
                self.needs_end = false;
                self.prev = self.first;
                Some(PathEvent::End {
                    last: self.prev,
                    first: self.first,
                    close: true,
                })
            }
            None => {
                if self.needs_end {
                    self.needs_end = false;
                    let last = self.prev;
                    let first = self.first;
                    Some(PathEvent::End {
                        last,
                        first,
                        close: false,
                    })
                } else {
                    None
                }
            }
        }
    }
}

struct VertexCtor {
    original_size: Size,
    pub scale: f32,
    pub style_index: u32,
}

// A 2x3 matrix (last two members of data1 unused).
#[repr(C)]
#[derive(Copy, Clone)]
struct GpuTransform {
    pub data0: [f32; 4],
    pub data1: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct GpuPrimitive {
    pub transform: u32,
    pub color: u32,
    pub _pad: [u32; 2],
}

impl GpuPrimitive {
    pub fn new(transform_idx: u32, color: usvg::Color, alpha: f32) -> Self {
        GpuPrimitive {
            transform: transform_idx,
            color: ((color.red as u32) << 24)
                + ((color.green as u32) << 16)
                + ((color.blue as u32) << 8)
                + (alpha * 255.0) as u32,
            _pad: [0; 2],
        }
    }
}

impl VertexCtor {
    fn to_shape_vertex(&self, position: &Point, normal: &Vector) -> ShapeVertex {
        ShapeVertex {
            position: [
                (position.x - self.original_size.width() / 2.0) * self.scale,
                // Y should be flipped since mercator coords are flipped
                (self.original_size.height() / 2.0 - position.y) * self.scale,
                0.0,
            ],
            normals: [normal.x, normal.y, 0.0],
            dist: 0.0, // TODO If we want to have dashed style for SVG
            style_index: self.style_index,
        }
    }
}

impl FillVertexConstructor<ShapeVertex> for VertexCtor {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex) -> ShapeVertex {
        self.to_shape_vertex(&vertex.position(), &Vector::new(0.0, 0.0))
    }
}

impl StrokeVertexConstructor<ShapeVertex> for VertexCtor {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> ShapeVertex {
        self.to_shape_vertex(&vertex.position(), &vertex.normal())
    }
}
