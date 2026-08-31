use geo_types::Point;
use glam::DVec3;
use lyon::geom::point;
use lyon::lyon_tessellation::{LineCap, LineJoin};
use lyon::path::Path;
use renderer_common::geometry_data::{GeometryData, GeometryType, PolylineOptions, ShapeData, StyledRangeInfo};
use renderer_common::render_group::RenderGroup;
use renderer_common::style_id::StyleId;
use renderer_common::CanvasApi;

pub struct TrackGroup {
    points: Vec<Point>,
}

impl TrackGroup {
    pub fn new(points: Vec<Point>) -> Self {
        TrackGroup { points }
    }

    pub fn first_point(&self) -> DVec3 {
        DVec3::new(self.points[0].x(), self.points[0].y(), 0.0)
    }
}

impl<T: CanvasApi> RenderGroup<T> for TrackGroup {
    fn content(&mut self, canvas: &mut T) {
        canvas.set_feature_layer_tag(Some("kml_layer".to_string()));

        let first = self.points[0];
        let mut path_builder = Path::builder();
        path_builder.begin(point(0.0f32, 0.0f32));

        for &p in self.points[1..].iter() {
            path_builder.line_to(point(
                (p.x() - first.x()) as f32,
                (p.y() - first.y()) as f32,
            ));
        }
        path_builder.end(false);

        let options = PolylineOptions {
            width: 1.2f32,
            line_join: LineJoin::Round,
            line_cap: LineCap::Round,
            tolerance: 0.01f32,
        };

        canvas.geometry_data(GeometryData::Shape(ShapeData {
            path: path_builder.build(),
            geometry_type: GeometryType::Polyline(options),
            style_id: StyleId::new("track"),
            index_layer_level: 0,
            styled_range_info: StyledRangeInfo::default(),
        }));
    }
}
