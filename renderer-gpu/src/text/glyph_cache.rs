use crate::buffer_pool::BufferPool;
use crate::global_context::GlobalContext;
use crate::mesh::mesh::Mesh;
use crate::text::default_face_wrapper::DefaultFaceWrapper;
use crate::text::glyph_tesselator::GlyphTesselator;
use crate::text::text_renderer::GlyphData;
use crate::vertex_attrs::MeshVertexWithUV;
use lyon::lyon_tessellation::VertexBuffers;
use renderer_common::geometry_data::StyledRangeInfo;
use rustc_hash::FxHashMap;
use rustybuzz::ttf_parser::GlyphId;
use std::ops::Range;
use std::sync::Arc;
use wgpu::Color;

pub(crate) struct GlyphCache {
    face: Arc<DefaultFaceWrapper>,
    mesh: Option<Mesh>,
    vb: VertexBuffers<MeshVertexWithUV, u32>,
    glyph_mesh_range_map: FxHashMap<GlyphId, Range<u32>>,
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
            mesh: None,
            vb: VertexBuffers::new(),
            glyph_mesh_range_map: FxHashMap::default(),
        }
    }

    pub fn process_glyph_data(
        &mut self,
        global_context: &GlobalContext,
        buffer_pool: &mut BufferPool,
        glyph_data: FxHashMap<GlyphId, Vec<GlyphData>>,
        action: impl FnOnce(&Mesh, Vec<(&GlyphId, &Range<u32>)>),
    ) {
        let mut path_builder: Option<GlyphTesselator> = None;
        let prev_mesh_state = (self.vb.vertices.len(), self.vb.indices.len());
        glyph_data.iter().for_each(|(glyph_id, _)| {
            self.glyph_mesh_range_map
                .entry(glyph_id.clone())
                .or_insert_with(|| {
                    let start_index = self.vb.indices.len() as u32;
                    let path_builder = path_builder
                        .get_or_insert_with(|| GlyphTesselator::new(DefaultFaceWrapper::MAX_SCALE));
                    self.face.outline_glyph(glyph_id.clone(), path_builder);
                    let path = path_builder.create_path();
                    path_builder.tessellate_stroke(&mut self.vb, &path, 4.0, Self::HALO_COLOR);
                    path_builder.tessellate_fill(&mut self.vb, &path, Self::PRIMARY_COLOR);

                    let end_index = self.vb.indices.len() as u32;

                    start_index..end_index
                });
        });

        let new_mesh_state = (self.vb.vertices.len(), self.vb.indices.len());
        if prev_mesh_state != new_mesh_state {
            println!("asd");
            let updated_mesh = Mesh::create(
                None,
                global_context,
                buffer_pool,
                &self.vb,
                StyledRangeInfo::default(),
            );
            self.mesh = Some(updated_mesh);
        }

        action(
            self.mesh.as_ref().expect("Should be created!"),
            self.glyph_mesh_range_map.iter().collect(),
        );
    }
}
