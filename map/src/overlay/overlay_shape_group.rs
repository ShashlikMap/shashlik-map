use geo::{Distance, Euclidean};
use geo_types::Point;
use glam::{DVec3, Vec2, Vec3};
use lyon::geom::{point, Box2D};
use lyon::geom::euclid::{point2, Size2D};
use lyon::lyon_tessellation::{LineCap, LineJoin};
use lyon::path::{Path, Winding};
use renderer_common::CanvasApi;
use renderer_common::geometry_data::{GeometryData, GeometryType, IconType, PolylineOptions, ShapeData, StyledRangeInfo, IconBackground, IconShapeData, IconData};
use renderer_common::render_group::RenderGroup;
use renderer_common::render_modifier::SpatialData;
use renderer_common::style_id::StyleId;
use crate::overlay::ShapeType;

pub struct OverlayShapeGroup {
    shape: Vec<Point>,
    feature_layer_tag: String,
    style_id: StyleId,
    shape_type: ShapeType,
}

impl OverlayShapeGroup {
    pub fn new(
        shape: Vec<Point>,
        feature_layer_tag: String,
        style_id: StyleId,
        shape_type: ShapeType,
    ) -> OverlayShapeGroup {
        OverlayShapeGroup {
            shape,
            feature_layer_tag,
            style_id,
            shape_type,
        }
    }

    pub fn spatial_data(&self) -> SpatialData {
        let point = DVec3::new(self.shape[0].x(), self.shape[0].y(), 0.0);
        SpatialData::transform(point)
    }
}

impl<T: CanvasApi> RenderGroup<T> for OverlayShapeGroup {
    fn content(&mut self, canvas: &mut T) {
        canvas.set_feature_layer_tag(Some(self.feature_layer_tag.clone()));
        let first_shape_point = self.shape[0];

        if matches!(self.shape_type, ShapeType::DottedLine) {
            let mut dist = 0f32;
            let mut sum_line_dist = 0f32;
            let mut point = self.shape.remove(0);
            let mut prev_point = point.clone();
            let mut vect: Option<Vec2> = None;
            loop {
                while !self.shape.is_empty() && dist > sum_line_dist {
                    let new_point = self.shape.remove(0);
                    let vect_point = new_point - point;
                    vect = Some(Vec2::new(vect_point.x() as f32, vect_point.y() as f32));
                    let d = Euclidean.distance(point, new_point);
                    prev_point = point;
                    point = new_point;
                    sum_line_dist += d as f32;
                }
                if let Some(vect) = vect {
                    let koef = (dist - (sum_line_dist - vect.length())) / vect.length();
                    let pos = vect * koef;
                    let size = 2.5;
                    canvas.geometry_data(GeometryData::Svg(IconShapeData {
                        id: 0,
                        icon_data: IconData {
                            id: "shape_dot",
                            icon_type: IconType::None
                        },
                        position: Vec3::new(
                            (prev_point.x() - first_shape_point.x()) as f32 + pos.x,
                            (prev_point.y() - first_shape_point.y()) as f32 + pos.y,
                            0.0,
                        )
                        .as_dvec3(),
                        size,
                        with_collision: false,
                        background: Some(IconBackground {
                            style_id: self.style_id.clone(),
                            shape: Box::new(move |_| {
                                let mut path_builder = Path::builder();
                                let bb = Box2D::from_origin_and_size(
                                    point2(-size * 0.5, -size * 0.5),
                                    Size2D::splat(size));
                                path_builder.add_rectangle(&bb, Winding::Negative);
                                path_builder.build()
                            }),
                        }),
                    }));
                }

                dist += 2f32;
                if self.shape.is_empty() && dist > sum_line_dist {
                    break;
                }
            }
        } else {
            let mut path_builder = Path::builder();
            path_builder.begin(point(0.0f32, 0.0f32));

            // TODO Should relative coords calc for the shape be the shape responsibility?
            for &p in self.shape[1..].iter() {
                path_builder.line_to(point(
                    (p.x() - first_shape_point.x()) as f32,
                    (p.y() - first_shape_point.y()) as f32,
                ));
            }
            let is_polygon = matches!(self.shape_type, ShapeType::Polygon);
            path_builder.end(is_polygon);

            let geometry_type = match self.shape_type {
                ShapeType::Line(width) => {
                    let options = PolylineOptions {
                        width: width.unwrap_or(1.0),
                        line_join: LineJoin::Round,
                        line_cap: LineCap::Round,
                        tolerance: 0.01f32, // this gives more or less a good round shape for join and caps
                    };
                    GeometryType::Polyline(options)
                },
                ShapeType::Polygon => {
                    GeometryType::Polygon
                },
                ShapeType::DottedLine => {
                    let options = PolylineOptions {
                        width: 1.0f32,
                        line_join: LineJoin::Round,
                        line_cap: LineCap::Round,
                        tolerance: 0.01f32, // this gives more or less a good round shape for join and caps
                    };
                    GeometryType::Polyline(options)
                }
            };

            canvas.geometry_data(GeometryData::Shape(ShapeData {
                path: path_builder.build(),
                geometry_type,
                style_id: self.style_id.clone(),
                index_layer_level: 0,
                styled_range_info: StyledRangeInfo::new(1, true)
            }));
        }
    }
}
