use crate::mesh::mesh::{Mesh, StyledRangeInfo};
use crate::text::default_face_wrapper::DefaultFaceWrapper;
use crate::text::glyph_tesselator::GlyphTesselator;
use lyon::lyon_tessellation::VertexBuffers;
use rustc_hash::FxHashMap;
use rustybuzz::ttf_parser::GlyphId;
use std::sync::Arc;
use wgpu::{Color, Device, Queue};
use crate::buffer_pool::BufferPool;

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

    pub fn get_or_tessellate(&mut self, device: &Device, queue: &Queue, buffer_pool: &mut BufferPool, glyph_id: &GlyphId) -> &Mesh {
        let glyph_id = glyph_id.clone();

        let mesh = self.glyph_mesh_map.entry(glyph_id).or_insert_with(|| {
            let mut path_builder = GlyphTesselator::new(DefaultFaceWrapper::MAX_SCALE);
            self.face.outline_glyph(glyph_id, &mut path_builder);
            let mut buffer = VertexBuffers::new();
            let path = path_builder.create_path();
            path_builder.tessellate_stroke(&mut buffer, &path, 4.0, Color::WHITE);
            path_builder.tessellate_fill(&mut buffer, &path, Color::BLACK);
            Mesh::create(None, device, queue, buffer_pool, &buffer, StyledRangeInfo(0, ""))
        });

        mesh
    }
}
