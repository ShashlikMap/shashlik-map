use crate::buffer_pool::BufferPool;
use crate::global_context::GlobalContext;
use crate::mesh::mesh::{Mesh};
use crate::text::default_face_wrapper::DefaultFaceWrapper;
use crate::text::glyph_tesselator::GlyphTesselator;
use lyon::lyon_tessellation::VertexBuffers;
use rustc_hash::FxHashMap;
use rustybuzz::ttf_parser::GlyphId;
use std::sync::Arc;
use wgpu::Color;
use renderer_common::geometry_data::StyledRangeInfo;

pub(crate) struct GlyphCache {
    face: Arc<DefaultFaceWrapper>,
    glyph_mesh_map: FxHashMap<GlyphId, Mesh>,
}

impl GlyphCache {

    const PRIMARY_COLOR: Color = Color {
        r: 0.2902,
        g: 0.2902,
        b: 0.2902,
        a: 1.0,
    };
    const HALO_COLOR: Color = Color {
        r: 0.9569,
        g: 0.9529,
        b: 0.9412,
        a: 0.9,
    };
    pub fn new(face: Arc<DefaultFaceWrapper>) -> Self {
        GlyphCache {
            face,
            glyph_mesh_map: FxHashMap::default(),
        }
    }

    pub fn get_or_tessellate(&mut self, global_context: &GlobalContext, buffer_pool: &mut BufferPool, glyph_id: &GlyphId) -> &Mesh {
        let glyph_id = glyph_id.clone();

        let mesh = self.glyph_mesh_map.entry(glyph_id).or_insert_with(|| {
            let mut path_builder = GlyphTesselator::new(DefaultFaceWrapper::MAX_SCALE);
            self.face.outline_glyph(glyph_id, &mut path_builder);
            let mut buffer = VertexBuffers::new();
            let path = path_builder.create_path();
            path_builder.tessellate_stroke(&mut buffer, &path, 4.0, Self::HALO_COLOR);
            path_builder.tessellate_fill(&mut buffer, &path, Self::PRIMARY_COLOR);
            Mesh::create(None, global_context, buffer_pool, &buffer, StyledRangeInfo(0, ""))
        });

        mesh
    }
}
