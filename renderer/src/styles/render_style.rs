use crate::consts::STYLE_SHADER_PARAMS_COUNT;

#[derive(Clone, Copy, Debug)]
pub struct RenderStyle {
    container: [f32; STYLE_SHADER_PARAMS_COUNT],
}

impl Default for RenderStyle {
    fn default() -> RenderStyle {
        RenderStyle::fill([1.0, 0.0, 0.0, 1.0])
    }
}

// TODO Builder
impl RenderStyle {
    fn empty() -> Self {
        RenderStyle {
            container: [0.0; STYLE_SHADER_PARAMS_COUNT],
        }
    }
    pub fn fill(fill_color: [f32; 4]) -> RenderStyle {
        let mut style = Self::empty();

        style.container[0] = 0.0;
        style.container[1..fill_color.len() + 1].copy_from_slice(&fill_color);

        style
    }

    pub fn border(fill_color: [f32; 4], darken_percent: f32) -> RenderStyle {
        let mut style = RenderStyle::fill(fill_color);

        style.container[0] = 1.0;
        style.container[5] = darken_percent;

        style
    }

    pub(crate) fn params(&self) -> [f32; STYLE_SHADER_PARAMS_COUNT] {
        self.container
    }
}
