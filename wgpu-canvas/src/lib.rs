use crate::geometry_data::GeometryData;
use crate::render_group::RenderGroup;
use crate::render_modifier::SpatialData;
use crate::render_style::RenderStyle;
use crate::style_id::StyleId;
use geo_types::Coord;
use glam::{dvec3, DMat4, DVec2, DVec3};
use std::collections::HashSet;
use std::sync::Arc;
use strum::{Display, EnumIter, EnumString};

mod consts;
pub mod geometry_data;
pub mod render_group;
pub mod render_modifier;
pub mod render_style;
pub mod style_id;
pub mod wgpu_canvas;

/// should be the same as mesh_shader.wgsl
pub static LIGHT_POS: DVec3 = dvec3(0.84, 1.12, 1.42);

pub fn feature_layer_tags() -> Vec<WorldShapeFeatureLayerTag> {
    vec![
        WorldShapeFeatureLayerTag {
            name: "kml_layer",
            ..Default::default()
        },
        WorldShapeFeatureLayerTag {
            name: "route_layer",
            vertex_shader: Some("vs_main_route"),
            indirect: true,
        },
        WorldShapeFeatureLayerTag {
            name: "puck_layer",
            ..Default::default()
        },
    ]
}

#[derive(Default)]
pub struct WorldShapeFeatureLayerTag {
    pub name: &'static str,
    pub vertex_shader: Option<&'static str>,
    pub indirect: bool,
}

pub struct RendererUpdateData {
    pub view_matrix: DMat4,
    pub view_light_matrix: DMat4,
    pub proj_matrix: DMat4,
    pub view_proj_matrix: DMat4,
    pub cs_offset: DVec3,
    pub scale: f32,
    pub eye_direction: DVec3,
    pub up: DVec3,
    pub scale_2d_3d: f32,
}

pub trait Renderer<T: MyRendererApi> {
    type OUTPUT;
    fn screen_size(&self) -> (f32, f32);
    fn resize(&mut self, width: u32, height: u32);
    fn update(&mut self, data: RendererUpdateData);
    fn clip_to_world(&self, coord: &Coord) -> Option<DVec2>;
    fn render(&mut self) -> Option<Self::OUTPUT>;

    fn api(&self) -> Arc<T>;
}

pub trait MyCanvasApi {
    fn set_feature_layer_tag(&mut self, tag: Option<String>);

    fn geometry_data(&mut self, geometry_data: GeometryData);
}

pub trait MyRendererApi: Send + Sync {
    type CANVAS: MyCanvasApi;
    fn add_render_group(
        &self,
        key: String,
        spatial_data: SpatialData,
        group: Box<dyn RenderGroup<Self::CANVAS>>,
    );

    fn clear_render_groups(&self, keys: HashSet<String>);

    fn update_style<F: FnOnce(&mut RenderStyle) + Send + 'static>(
        &self,
        style_id: StyleId,
        updater: F,
    );

    fn update_spatial_data<F: FnOnce(&mut SpatialData) + Send + 'static>(
        &self,
        key: String,
        updater: F,
    );
}

// TODO Proper config manager

#[derive(Eq, PartialEq, Copy, Clone, Hash, Display, EnumIter, EnumString)]
pub enum PreviewType {
    None,
    Camera,
    SSAO,
    SSAOPositions,
    SSAONormals,
    SSAODepth,
    ShadowMap,
}

impl PreviewType {
    pub fn is_enabled(self) -> bool {
        self != PreviewType::None
    }
}

pub static mut SHADOWS_ENABLED: bool = true;
pub static mut SHADOWS_TEX_SIZE: (u32, u32) = (2048, 2048);
pub static mut SSAO_ENABLED: bool = true;
pub static mut PREVIEW_TYPE: PreviewType = PreviewType::None;
