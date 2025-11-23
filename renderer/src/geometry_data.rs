use lyon::path::Path;
use cgmath::{Vector2, Vector3};
use geo_types::Coord;
use lyon::lyon_tessellation::VertexBuffers;
use crate::draw_commands::{GeometryType, MeshVertex};
use crate::styles::style_id::StyleId;

pub enum GeometryData {
    Shape(ShapeData),
    ExtrudedPolygon(ExtrudedPolygonData),
    Mesh3d(Mesh3d),
    Svg(SvgData),
    Text(TextData),
}

#[derive(Clone)]
pub struct ShapeData {
    pub path: Path,
    pub geometry_type: GeometryType,
    pub style_id: StyleId,
    pub index_layer_level: i8,
    pub is_screen: bool // might not be the best idea
}

#[derive(Clone)]
pub struct ExtrudedPolygonData {
    pub path: Path,
    pub height: f32,
}

#[derive(Clone)]
pub struct Mesh3d {
    pub mesh_data: VertexBuffers<MeshVertex, u32>,
}

#[derive(Clone)]
pub struct SvgData {
    pub icon: (&'static str, &'static [u8]),
    pub position: Vector3<f64>,
    pub size: f32,
    pub style_id: StyleId,
    pub with_collision: bool
}

#[derive(Clone)]
pub struct TextData {
    pub id: u64,
    pub text: String,
    pub position: Vector3<f32>,
    pub screen_offset: Vector2<f32>,
    pub size: f32,
    pub positions: Option<Vec<Coord>>
}