use renderer_common::PreviewType;

pub struct RenderConfig {
    pub shadow_enabled: bool,
    pub x_real_mesh_shader_enabled: bool,
    shadow_texture_size: u32,
    pub ssao_enabled: bool,
    pub preview_type: PreviewType,
    pub headless: bool,
    round_screen: bool
}

impl RenderConfig {
    pub const DEFAULT_SHADOW_TEX_SIZE: u32 = 2048;
    pub const HALF_SHADOW_TEX_SIZE: u32 = 1024;

    pub fn new(shadow_texture_size: u32, round_screen: bool) -> RenderConfig {
        assert!(shadow_texture_size > 0);
        let mut config = RenderConfig::default();
        config.shadow_texture_size = shadow_texture_size;
        config.round_screen = round_screen;
        config
    }

    pub fn shadow_texture_size(&self) -> (u32, u32) {
        (self.shadow_texture_size, self.shadow_texture_size)
    }

    pub fn round_screen(&self) -> bool {
        self.round_screen
    }
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            shadow_enabled: true,
            x_real_mesh_shader_enabled: false,
            shadow_texture_size: Self::DEFAULT_SHADOW_TEX_SIZE,
            ssao_enabled: false,
            preview_type: PreviewType::None,
            headless: false,
            round_screen: false
        }
    }
}
