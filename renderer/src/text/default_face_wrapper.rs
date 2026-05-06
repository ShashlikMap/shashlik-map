use crate::mesh::mesh::{Mesh, StyledRangeInfo};
use crate::text::glyph_tesselator::GlyphTesselator;
use glam::{Mat4, Vec2, Vec3};
use rustc_hash::FxHashMap;
use rustybuzz::ttf_parser::GlyphId;
use rustybuzz::{Face, GlyphBuffer, UnicodeBuffer};
use wgpu::{Color, Device};

#[derive(Clone)]
pub struct FaceTextParams {
    pub scale_matrix: Mat4,
    pub half_height_translation: Mat4,
    pub width: f32,
    pub height: f32,
    pub scale: f32,
}

pub struct DefaultFaceWrapper {
    face: Face<'static>,
    pub glyph_height: f32,
}

impl DefaultFaceWrapper {
    const MAX_SCALE: f32 = 0.035;
    pub fn new(font: &'static rustybuzz::ttf_parser::Face) -> DefaultFaceWrapper {
        let face = rustybuzz::Face::from_face(font.clone());
        let glyph_height = (face.ascender() + face.descender()) as f32;

        DefaultFaceWrapper {
            face,
            glyph_height,
        }
    }

    fn get_scale_by_font_size(&self, font_size: f32) -> f32 {
        let units = self.face.units_per_em() as f32;
        font_size / units
    }

    pub fn shape(&self, text: &str) -> GlyphBuffer {
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        rustybuzz::shape(&self.face, &[], buffer)
    }

    pub fn get_or_tessellate<'a>(&self, device: &Device, glyph_id: &GlyphId, cache: &'a mut FxHashMap<GlyphId, Mesh>) -> &'a mut Mesh {
        let glyph_id = glyph_id.clone();
        let mesh = cache.entry(glyph_id).or_insert_with(|| {
            let mut path_builder = GlyphTesselator::new(Self::MAX_SCALE);
            self.face.outline_glyph(glyph_id, &mut path_builder);
            let glyph_buf = path_builder.tessellate_fill(Vec2::new(0.0, 0.0f32), Color::RED);
            Mesh::create(&device, &glyph_buf, StyledRangeInfo(0, ""))
        });

        mesh
    }

    pub fn get_text_params(
        &self,
        glyph_buffer: &GlyphBuffer,
        font_size: f32,
    ) -> FaceTextParams {
        let scale = self.get_scale_by_font_size(font_size);

        let width = glyph_buffer
            .glyph_positions()
            .iter()
            .fold(0, |aggr, glyph| aggr + glyph.x_advance) as f32
            * scale;
        let height = self.glyph_height * scale;

        let scale_matrix = Mat4::from_scale(Vec3::splat(scale / Self::MAX_SCALE));

        let half_height_translation =
            Mat4::from_translation(Vec3::new(0.0, -height / 2.0, 0.0));
        
        FaceTextParams {
            scale_matrix,
            half_height_translation,
            width,
            height,
            scale,
        }
    }
}
