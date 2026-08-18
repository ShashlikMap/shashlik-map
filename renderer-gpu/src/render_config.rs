use renderer_common::PreviewType;

pub struct RenderConfig {
    pub shadow_enabled: bool,
    pub x_real_mesh_shader_enabled: bool,
    shadow_texture_size: u32,
    pub ssao_enabled: bool,
    pub preview_type: PreviewType,
}

impl RenderConfig {
    pub fn new(shadow_texture_size: u32) -> RenderConfig {
        assert!(shadow_texture_size > 0);
        let mut config = RenderConfig::default();
        config.shadow_texture_size = shadow_texture_size;
        config
    }

    pub fn shadow_texture_size(&self) -> (u32, u32) {
        (self.shadow_texture_size, self.shadow_texture_size)
    }
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            shadow_enabled: true,
            x_real_mesh_shader_enabled: false,
            shadow_texture_size: 2048,
            ssao_enabled: false,
            preview_type: PreviewType::None,
        }
    }
}
