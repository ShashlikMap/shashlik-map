use glam::{DVec3, Vec2};
use crate::draw_commands::{GeometryType, MeshVertex};
use crate::styles::style_id::StyleId;
use lyon::lyon_tessellation::VertexBuffers;
use lyon::path::Path;
use rustybuzz::GlyphBuffer;
use crate::mesh::mesh::StyledRangeInfo;

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
    pub styled_range_info: StyledRangeInfo
}

pub struct ExtrudedPolygonData {
    pub path: Path,
    pub height: f32,
}

pub struct Mesh3d {
    pub mesh_data: VertexBuffers<MeshVertex, u32>,
}

pub struct SvgBackground {
    pub style_id: StyleId,
    pub padding: f32
}

pub struct SvgData {
    pub icon: (&'static str, &'static [u8]),
    pub position: DVec3,
    pub size: f32,
    pub style_id: StyleId,
    pub with_collision: bool,
    pub background: Option<SvgBackground>,
}

pub struct TextData {
    pub id: u64,
    pub text: String,
    pub screen_offset: Vec2,
    pub size: f32,
    pub(crate) alpha: f32,
    pub positions: Vec<DVec3>,
    pub(crate) screen_space: bool,
    pub(crate) glyph_buffer: Option<GlyphBuffer>,
}

impl TextData {
    pub fn new(
        id: u64,
        text: String,
        screen_offset: Vec2,
        size: f32,
        positions: Vec<DVec3>,
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

    pub fn update_text(&mut self, new_text: &str, alpha: f32) {
        self.text = new_text.to_string();
        self.alpha = alpha;
        // clear buffer so render can re-create it
        self.glyph_buffer = None;
    }
}
