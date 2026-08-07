use renderer_common::PreviewType;

pub struct RenderConfig {
    pub shadow_enabled: bool,
    pub shadow_texture_size: (u32, u32),
    pub ssao_enabled: bool,
    pub preview_type: PreviewType,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            shadow_enabled: true,
            shadow_texture_size: (2048, 2048),
            ssao_enabled: false,
            preview_type: PreviewType::None,
        }
    }
}
