pub mod style_loader;

use serde::{Deserialize};
#[derive(Deserialize)]
pub struct FeatureStyle {
    pub id: String,
    #[serde(rename(deserialize = "render_style"))]
    pub feature_style: FeatureStyleType,
}

#[derive(Deserialize)]
pub enum FeatureStyleType {
    Fill(FeatureStyleColor),
    Border(FeatureStyleColor, f32),
    Dashed(FeatureStyleColor, FeatureStyleColor, DashStyle),
}

#[derive(Deserialize)]
pub enum DashStyle {
    Solid,
    Circles
}

#[derive(Deserialize)]
pub struct FeatureStyleColor {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl FeatureStyleColor {
    pub fn as_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}
