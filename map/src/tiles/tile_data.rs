use glam::DVec3;
use renderer::geometry_data::GeometryData;

pub struct TileData {
    pub key: String,
    pub position: DVec3,
    pub size: (f64, f64),
    pub geometry_data: Vec<GeometryData>,
}
