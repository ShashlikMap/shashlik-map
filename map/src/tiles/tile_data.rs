use cgmath::Vector3;
use renderer::geometry_data::GeometryData;

pub struct TileData {
    pub key: String,
    pub position: Vector3<f32>,
    pub geometry_data: Vec<GeometryData>,
}
