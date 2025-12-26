use cgmath::Vector3;
use geo_types::Point;
use lyon::geom::point;
use lyon::lyon_tessellation::{LineCap, LineJoin};
use lyon::path::Path;
use renderer::canvas_api::CanvasApi;
use renderer::draw_commands::{GeometryType, PolylineOptions};
use renderer::geometry_data::ShapeData;
use renderer::render_group::RenderGroup;
use renderer::styles::style_id::StyleId;
use crate::route::RouteCosting;

pub struct RouteGroup {
    route: Vec<Point>,
    route_costing: RouteCosting
}

impl RouteGroup {
    pub fn new(route: Vec<Point>, route_costing: RouteCosting, converter: Box<dyn Fn(&Point) -> Point>) -> RouteGroup {
        let route: Vec<Point> = route.iter().map(|p| converter(p)).collect();
        RouteGroup { route, route_costing }
    }

    pub fn first_route_point(&self) -> Vector3<f64> {
        Vector3::new(self.route[0].x(), self.route[0].y(), 0.0)
    }
}

impl RenderGroup for RouteGroup {
    fn content(&mut self, canvas: &mut CanvasApi) {
        let mut path_builder = Path::builder();
        let first_route_point = self.route[0];
        path_builder.begin(point(0.0f32, 0.0f32));

        // TODO Should relative coords calc for the route be the route responsibility?
        for &p in self.route[1..].iter() {
            path_builder.line_to(point((p.x() - first_route_point.x()) as f32,
                                       (p.y() - first_route_point.y()) as f32));
        }
        path_builder.end(false);

        let options = PolylineOptions {
            width: 1f32,
            line_join: LineJoin::Round,
            line_cap: LineCap::Round,
            tolerance: 0.05f32,
        };
        
        let style_id = match self.route_costing {
            RouteCosting::Pedestrian =>  StyleId("route_pedestrian"),
            RouteCosting::Motorbike =>  StyleId("route_motorbike")
        };
        
        canvas.path(ShapeData {
            path: path_builder.build(),
            geometry_type: GeometryType::Polyline(options),
            style_id,
            index_layer_level: 0,
            is_screen: false,
        });
    }
}
