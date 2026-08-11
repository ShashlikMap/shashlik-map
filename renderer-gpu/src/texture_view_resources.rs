use crate::render_config::RenderConfig;
use crate::textures::create_depth_texture;
use rustc_hash::FxHashMap;
use wgpu::{Buffer, Device, TextureFormat, TextureView};

#[derive(Eq, PartialEq, Hash)]
pub(crate) enum TextureViewKind {
    GBufPositions,
    GBufNormals,
    GBufDepth,
    ShadowMapDepth,
    SSAO,
}

#[derive(Clone, Default)]
pub struct MeshBuffers {
    pub instance_buffer: Option<Buffer>,
    pub culled_buffer: Option<Buffer>,
    pub instance_args_buffer: Option<Buffer>,
}


pub(crate) struct TextureViewResources {
    textures: FxHashMap<TextureViewKind, TextureView>,
}

impl TextureViewResources {
    pub fn new(render_config: &RenderConfig, device: &Device) -> Self {
        // TODO Currently, mesh pipeline expects it to be created from beginning.
        //  It's going be handled later during layers/pipeline redesign.
        let shadow_map_depth_texture = create_depth_texture(
            render_config.shadow_texture_size(),
            1,
            TextureFormat::Depth32Float,
            device,
        );
        let mut textures = FxHashMap::default();
        textures.insert(TextureViewKind::ShadowMapDepth, shadow_map_depth_texture);
        Self {
            textures,
        }
    }

    pub fn insert(&mut self, texture_view_kind: TextureViewKind, texture: TextureView) {
        self.textures.insert(texture_view_kind, texture);
    }

    pub fn get(&self, texture_view_kind: TextureViewKind) -> Option<&TextureView> {
        self.textures.get(&texture_view_kind)
    }

    pub fn get_or_unwrap(&self, texture_view_kind: TextureViewKind) -> &TextureView {
        self.textures.get(&texture_view_kind).unwrap()
    }
}
