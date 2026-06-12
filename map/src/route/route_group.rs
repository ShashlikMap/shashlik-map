use crate::route::RouteCosting;
use geo::{Distance, Euclidean};
use geo_types::Point;
use glam::{DVec3, Vec2, Vec3};
use lyon::geom::point;
use lyon::lyon_tessellation::{LineCap, LineJoin};
use lyon::path::Path;
use renderer::canvas_api::CanvasApi;
use renderer::draw_commands::{GeometryType, PolylineOptions};
use renderer::geometry_data::{GeometryData, ShapeData, SvgData};
use renderer::mesh::mesh::StyledRangeInfo;
use renderer::render_group::RenderGroup;
use renderer::styles::style_id::StyleId;

pub struct RouteGroup {
    route: Vec<Point>,
    route_costing: RouteCosting,
}

impl RouteGroup {
    pub const SQUARE_SVG: &'static [u8] = include_bytes!("../../svg/just_square.svg");
    pub fn new(
        route: Vec<Point>,
        route_costing: RouteCosting,
    ) -> RouteGroup {
        RouteGroup {
            route,
            route_costing,
        }
    }

    pub fn first_route_point(&self) -> DVec3 {
        DVec3::new(self.route[0].x(), self.route[0].y(), 0.0)
    }
}

impl RenderGroup for RouteGroup {
    fn content(&mut self, canvas: &mut CanvasApi) {
        canvas.set_feature_layer_tag(Some("route_layer".to_string()));
        let first_route_point = self.route[0];

        let style_id = match self.route_costing {
            RouteCosting::Pedestrian => StyleId("route_pedestrian"),
            RouteCosting::Auto | RouteCosting::Motorbike => StyleId("route_motorbike"),
        };

        if matches!(self.route_costing, RouteCosting::Pedestrian) {
            let mut dist = 0f32;
            let mut sum_route_dist = 0f32;
            let mut point = self.route.remove(0);
            let mut prev_point = point.clone();
            let mut vect: Option<Vec2> = None;
            loop {
                while !self.route.is_empty() && dist > sum_route_dist {
                    let new_point = self.route.remove(0);
                    let vect_point = new_point - point;
                    vect = Some(Vec2::new(vect_point.x() as f32, vect_point.y() as f32));
                    let d = Euclidean.distance(point, new_point);
                    prev_point = point;
                    point = new_point;
                    sum_route_dist += d as f32;
                }
                if let Some(vect) = vect {
                    let koef = (dist - (sum_route_dist - vect.length())) / vect.length();
                    let pos = vect * koef;

                    canvas.geometry_data(GeometryData::Svg(SvgData {
                        // TODO shape instead of SVG
                        icon: ("route_dot", Self::SQUARE_SVG),
                        position: Vec3::new(
                            (prev_point.x() - first_route_point.x()) as f32 + pos.x,
                            (prev_point.y() - first_route_point.y()) as f32 + pos.y,
                            0.0,
                        ).as_dvec3(),
                        size: 2.5,
                        style_id: style_id.clone(),
                        with_collision: false,
                        background: None,
                    }));
                }

                dist += 2f32;
                if self.route.is_empty() && dist > sum_route_dist {
                    break;
                }
            }
        } else {
            let mut path_builder = Path::builder();
            path_builder.begin(point(0.0f32, 0.0f32));

            // TODO Should relative coords calc for the route be the route responsibility?
            for &p in self.route[1..].iter() {
                path_builder.line_to(point(
                    (p.x() - first_route_point.x()) as f32,
                    (p.y() - first_route_point.y()) as f32,
                ));
            }
            path_builder.end(false);

            let options = PolylineOptions {
                width: 1f32,
                line_join: LineJoin::Round,
                line_cap: LineCap::Round,
                tolerance: 0.01f32, // this gives more or less a good round shape for join and caps
            };

            canvas.geometry_data(GeometryData::Shape(ShapeData {
                path: path_builder.build(),
                geometry_type: GeometryType::Polyline(options),
                style_id,
                index_layer_level: 0,
                styled_range_info: StyledRangeInfo(0, ""),
            }));
        }
    }
}
