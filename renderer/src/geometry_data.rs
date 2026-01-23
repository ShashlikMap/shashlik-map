use crate::draw_commands::{GeometryType, MeshVertex};
use crate::styles::style_id::StyleId;
use cgmath::{Vector2, Vector3};
use lyon::lyon_tessellation::VertexBuffers;
use lyon::path::Path;
use rustybuzz::GlyphBuffer;

pub enum GeometryData {
    Shape(ShapeData),
    ExtrudedPolygon(ExtrudedPolygonData),
    Svg(SvgData),
    Text(TextData),
}

pub struct ShapeData {
    pub path: Path,
    pub geometry_type: GeometryType,
    pub style_id: StyleId,
    pub index_layer_level: i8,
}

pub struct ExtrudedPolygonData {
    pub path: Path,
    pub height: f32,
}

pub struct Mesh3d {
    pub mesh_data: VertexBuffers<MeshVertex, u32>,
}

pub struct SvgData {
    pub icon: (&'static str, &'static [u8]),
    pub position: Vector3<f64>,
    pub size: f32,
    pub style_id: StyleId,
    pub with_collision: bool,
}

pub struct TextData {
    pub id: u64,
    pub text: String,
    pub screen_offset: Vector2<f32>,
    pub size: f32,
    pub(crate) alpha: f32,
    pub positions: Vec<Vector3<f64>>,
    pub(crate) screen_space: bool,
    pub(crate) glyph_buffer: Option<GlyphBuffer>,
}

impl TextData {
    pub fn new(
        id: u64,
        text: String,
        screen_offset: Vector2<f32>,
        size: f32,
        positions: Vec<Vector3<f64>>,
    ) -> Self {
        Self {
            id,
            text,
            screen_offset,
            size,
            alpha: 1.0f32,
            positions,
            screen_space: false,
            glyph_buffer: None,
        }
    }
}
