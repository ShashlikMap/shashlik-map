#[derive(Clone, Copy, Debug)]
pub struct RenderStyle {
    header: [f32; 4],
    color_1: [f32; 4],
    color_2: [f32; 4],
}

impl Default for RenderStyle {
    fn default() -> RenderStyle {
        RenderStyle::fill([1.0, 0.0, 0.0, 1.0])
    }
}

impl RenderStyle {
    fn empty() -> Self {
        RenderStyle {
            header: [0.0; 4],
            color_1: [0.0; 4],
            color_2: [0.0; 4],
        }
    }
    pub fn fill(fill_color: [f32; 4]) -> RenderStyle {
        let mut style = Self::empty();

        style.header[0] = 0.0;
        style.color_1 = fill_color;

        style
    }

    pub fn get_fill_color(&self) -> [f32; 4] {
        self.color_1
    }

    pub fn border(fill_color: [f32; 4], darken_percent: f32) -> RenderStyle {
        let mut style = RenderStyle::fill(fill_color);

        style.header[0] = 1.0;
        style.header[1] = darken_percent;

        style
    }

    pub fn dashed(fill_color: [f32; 4], dash_color: [f32; 4], dash_style: u8) -> RenderStyle {
        let mut style = RenderStyle::fill(fill_color);

        style.header[0] = 2.0;
        style.header[1] = dash_style as f32;
        style.color_2 = dash_color;

        style
    }

    pub fn params(&self) -> Vec<[f32; 4]> {
        vec![self.header, self.color_1, self.color_2]
    }
}
