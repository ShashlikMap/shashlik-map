use crate::style_id::StyleId;
use glam::{DVec3, Mat4, Vec2};
use lyon::lyon_tessellation::{LineCap, LineJoin, VertexBuffers};
use lyon::path::Path;
use rustybuzz::GlyphBuffer;
use std::cell::OnceCell;

#[derive(Clone, Default)]
pub struct StyledRangeInfo {
    pub instance_offset: u8,
    pub skip_preview: bool,
    pub skip_after: Option<f32>
}

impl StyledRangeInfo {
    pub fn new(instance_offset: u8, skip_preview: bool) -> StyledRangeInfo {
        StyledRangeInfo {
            instance_offset,
            skip_preview,
            skip_after: None
        }
    }
}

#[derive(Clone)]
pub struct FaceTextParams {
    pub scale_matrix: Mat4,
    pub half_height_translation: Mat4,
    pub width: f32,
    pub height: f32,
    pub scale: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normals: [f32; 3],
}

#[derive(Clone, Copy)]
pub enum GeometryType {
    Polyline(PolylineOptions),
    Polygon,
}

#[derive(Clone, Copy)]
pub struct PolylineOptions {
    pub width: f32,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
    pub tolerance: f32,
}

impl Default for PolylineOptions {
    fn default() -> Self {
        PolylineOptions {
            width: 1f32,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            tolerance: 1f32,
        }
    }
}


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
    pub styled_range_info: StyledRangeInfo,
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
    pub padding: f32,
}

pub struct SvgData {
    pub icon: (&'static str, &'static [u8]),
    pub position: DVec3,
    pub size: f32,
    pub style_id: Option<StyleId>,
    pub with_collision: bool,
    pub background: Option<SvgBackground>,
}

pub struct LineData {
    pub positions: Vec<DVec3>,
    center_segment_index: OnceCell<usize>,
}

impl LineData {
    pub fn new(positions: Vec<DVec3>) -> Self {
        Self {
            positions,
            center_segment_index: OnceCell::new(),
        }
    }

    pub fn get_center_segment_index(&self) -> usize {
        *self.center_segment_index.get_or_init(|| {
            let positions_segments: Vec<_> = self
                .positions
                .windows(2)
                .map(|pair| pair[1] - pair[0])
                .collect();

            let positions_segments_sum = positions_segments
                .iter()
                .map(|it| it.length() as f32)
                .sum::<f32>();
            let sp0 = positions_segments_sum * 0.5;
            let mut temp_l = 0f32;
            positions_segments
                .iter()
                .position(|it| {
                    temp_l += it.length() as f32;
                    temp_l >= sp0
                })
                .unwrap_or(0)
        })
    }
}

pub struct TextData {
    pub id: u64,
    pub text: String,
    pub screen_offset: Vec2,
    pub size: f32,
    pub alpha: f32,
    pub line_data: LineData,
    pub screen_space: bool,
    pub glyph_buffer: Option<GlyphBuffer>,
    pub face_text_params: Option<FaceTextParams>,
}

impl TextData {
    pub fn new(
        id: u64,
        text: String,
        screen_offset: Vec2,
        size: f32,
        line_data: LineData,
    ) -> Self {
        Self {
            id,
            text,
            screen_offset,
            size,
            alpha: 1.0f32,
            line_data,
            screen_space: false,
            glyph_buffer: None,
            face_text_params: None,
        }
    }
    pub fn screen_space_new(
        id: u64,
        text: String,
        screen_offset: Vec2,
        size: f32,
        line_data: LineData,
    ) -> Self {
        Self {
            id,
            text,
            screen_offset,
            size,
            alpha: 1.0f32,
            line_data,
            screen_space: true,
            glyph_buffer: None,
            face_text_params: None,
        }
    }

    pub fn update_text(&mut self, new_text: &str, alpha: f32) {
        self.text = new_text.to_string();
        self.alpha = alpha;
        // clear buffer so render can re-create it
        self.glyph_buffer = None;
    }
}
