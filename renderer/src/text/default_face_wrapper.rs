use crate::draw_commands::geometry_to_mesh;
use crate::mesh::mesh::Mesh;
use crate::text::glyph_tesselator::GlyphTesselator;
use cgmath::{Matrix4, Vector2};
use log::error;
use rustc_hash::FxHashMap;
use rustybuzz::ttf_parser::GlyphId;
use rustybuzz::{Direction, Face, GlyphBuffer, ShapePlan, UnicodeBuffer, ttf_parser, Script};
use wgpu::{Color, Device};

pub struct DefaultFaceWrapper {
    face: Face<'static>,
    face_shape_plan: ShapePlan,
    plan_script: Script,
    pub glyph_mesh_map: FxHashMap<GlyphId, Mesh>,
    pub glyph_height: f32,
}

impl DefaultFaceWrapper {
    const MAX_SCALE: f32 = 0.035;
    pub fn new(device: &Device) -> DefaultFaceWrapper {
        let face = ttf_parser::Face::parse(include_bytes!("../font.ttf"), 0).unwrap();
        let face = rustybuzz::Face::from_face(face);

        let mut buffer = UnicodeBuffer::new();
        buffer.push_str("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-");
        buffer.guess_segment_properties();

        let plan_script = buffer.script();
        let face_shape_plan = ShapePlan::new(
            &face,
            Direction::LeftToRight,
            Some(plan_script),
            None,
            &[],
        );

        let glyph_buffer = rustybuzz::shape(&face, &[], buffer);

        let mut glyph_mesh_map = FxHashMap::default();
        for index in 0..glyph_buffer.len() {
            let glyph_info = glyph_buffer.glyph_infos()[index];
            let mut path_builder = GlyphTesselator::new(Self::MAX_SCALE);
            face.outline_glyph(GlyphId(glyph_info.glyph_id as u16), &mut path_builder);
            let glyph_buf = path_builder.tessellate_fill(Vector2::new(0.0, 0.0f32), Color::RED);
            glyph_mesh_map.insert(
                GlyphId(glyph_info.glyph_id as u16),
                geometry_to_mesh(device, &glyph_buf),
            );
        }

        let glyph_height = (face.ascender() + face.descender()) as f32;

        DefaultFaceWrapper {
            face,
            face_shape_plan,
            plan_script,
            glyph_mesh_map,
            glyph_height,
        }
    }

    fn check_scripts_with_buffer(&self, buffer: &mut UnicodeBuffer) -> bool {
        buffer.guess_segment_properties();
        buffer.script() == self.plan_script
    }

    fn get_scale_by_font_size(&self, font_size: f32) -> f32 {
        let units = self.face.units_per_em() as f32;
        font_size / units
    }

    pub fn shape(&self, text: &str) -> GlyphBuffer {
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);

        // FIXME So far, other languages than English are not supported, so the script check will fail in Debug
        if !self.check_scripts_with_buffer(&mut buffer) {
            error!("Failed to shape due to scripts are not equal, text: {}", text);
            // force set the script as the plan has
            buffer.set_script(self.plan_script);
        }
        rustybuzz::shape_with_plan(&self.face, &self.face_shape_plan, buffer)
    }

    pub fn get_text_params(
        &self,
        glyph_buffer: &GlyphBuffer,
        font_size: f32,
    ) -> (Matrix4<f32>, f32, f32, f32) {
        let scale = self.get_scale_by_font_size(font_size);

        let width = glyph_buffer
            .glyph_positions()
            .iter()
            .fold(0, |aggr, glyph| aggr + glyph.x_advance) as f32
            * scale;
        let height = self.glyph_height * scale;

        let scale_m = Matrix4::from_scale(scale / Self::MAX_SCALE);

        (scale_m, width, height, scale)
    }
}
