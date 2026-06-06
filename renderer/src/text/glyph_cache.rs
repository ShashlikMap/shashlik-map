use crate::mesh::mesh::{Mesh, StyledRangeInfo};
use crate::text::default_face_wrapper::DefaultFaceWrapper;
use crate::text::glyph_tesselator::GlyphTesselator;
use glam::Vec2;
use rustc_hash::FxHashMap;
use rustybuzz::ttf_parser::GlyphId;
use std::sync::Arc;
use wgpu::{Color, Device};

pub(crate) struct GlyphCache {
    face: Arc<DefaultFaceWrapper>,
    glyph_mesh_map: FxHashMap<GlyphId, Mesh>,
}

impl GlyphCache {
    pub fn new(face: Arc<DefaultFaceWrapper>) -> Self {
        GlyphCache {
            face,
            glyph_mesh_map: FxHashMap::default(),
        }
    }

    pub fn get_or_tessellate(&mut self, device: &Device, glyph_id: &GlyphId) -> &Mesh {
        let glyph_id = glyph_id.clone();

        let mesh = self.glyph_mesh_map.entry(glyph_id).or_insert_with(|| {
            let mut path_builder = GlyphTesselator::new(DefaultFaceWrapper::MAX_SCALE);
            self.face.outline_glyph(glyph_id, &mut path_builder);
            let glyph_buf = path_builder.tessellate_fill(Vec2::new(0.0, 0.0f32), Color::BLUE);
            Mesh::create(&device, &glyph_buf, StyledRangeInfo(0, ""))
        });

        mesh
    }
}
