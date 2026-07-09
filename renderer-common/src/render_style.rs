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

    pub fn dashed(fill_color: [f32; 4], dash_color: [f32; 4], dash_style: u8) -> RenderStyle {
        let mut style = RenderStyle::fill(fill_color);

        style.container[0] = 2.0;
        style.container[5..dash_color.len() + 5].copy_from_slice(&dash_color);
        style.container[9] = dash_style as f32;

        style
    }

    pub fn params(&self) -> [[f32; 4]; 4] {
        Self::convert_to_wgsl_mat4x3(self.container)
    }

    pub fn set_alpha(&mut self, alpha: f32) {
        self.container[4] = alpha;
    }

    fn convert_to_wgsl_mat4x3(flat_array: [f32; 12]) -> [[f32; 4]; 4] {
        [
            [flat_array[0], flat_array[1], flat_array[2], 0.0],  // Column 0
            [flat_array[3], flat_array[4], flat_array[5], 0.0],  // Column 1
            [flat_array[6], flat_array[7], flat_array[8], 0.0],  // Column 2
            [flat_array[9], flat_array[10], flat_array[11], 0.0], // Column 3
        ]
    }
}
