use geo_types::Point;

pub(crate) mod overlay_shape_group;
pub mod overlay;

pub enum ShapeType {
    Line(Option<f32>),
    Polygon,
    DottedLine,
}

impl ShapeType {
    fn are_points_valid(&self, points: &Vec<Point>) -> bool {
        match &self {
            ShapeType::Line(_) | ShapeType::DottedLine => points.len() >= 2,
            ShapeType::Polygon => points.len() >= 1
        }
    }
}