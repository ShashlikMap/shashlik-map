use geo_types::Rect;
use glam::DVec3;
use renderer_common::geometry_data::GeometryData;

pub struct TileData {
    pub key: String,
    pub position: DVec3,
    pub zoom_level: i32,
    pub bbox: Rect,
    pub geometry_data: Vec<GeometryData>,
}
