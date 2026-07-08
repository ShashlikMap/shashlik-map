use glam::{Mat4, Vec3};
use rustybuzz::ttf_parser::{GlyphId, OutlineBuilder};
use rustybuzz::{Face, GlyphBuffer, UnicodeBuffer};
use wgpu_canvas::geometry_data::FaceTextParams;

pub struct DefaultFaceWrapper {
    face: Face<'static>,
    glyph_height: f32,
}

impl DefaultFaceWrapper {
    pub const MAX_SCALE: f32 = 0.035;
    pub fn new(font: &'static rustybuzz::ttf_parser::Face) -> DefaultFaceWrapper {
        let face = rustybuzz::Face::from_face(font.clone());
        let glyph_height = (face.ascender() + face.descender()) as f32;

        DefaultFaceWrapper { face, glyph_height }
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

    pub fn outline_glyph(&self, glyph_id: GlyphId, builder: &mut dyn OutlineBuilder) {
        self.face.outline_glyph(glyph_id, builder);
    }

    pub fn get_text_params(&self, glyph_buffer: &GlyphBuffer, font_size: f32) -> FaceTextParams {
        let scale = self.get_scale_by_font_size(font_size);

        let width = glyph_buffer
            .glyph_positions()
            .iter()
            .fold(0, |aggr, glyph| aggr + glyph.x_advance) as f32
            * scale;
        let height = self.glyph_height * scale;

        let scale_matrix = Mat4::from_scale(Vec3::splat(scale / Self::MAX_SCALE));

        let half_height_translation = Mat4::from_translation(Vec3::new(0.0, -height / 2.0, 0.0));

        FaceTextParams {
            scale_matrix,
            half_height_translation,
            width,
            height,
            scale,
        }
    }
}
