use crate::geometry_data::GeometryData;
use crate::render_group::RenderGroup;
use crate::render_modifier::SpatialData;
use crate::render_style::RenderStyle;
use crate::style_id::StyleId;
use geo_types::Coord;
use glam::{DMat4, DVec2, DVec3, dvec3};
use std::collections::HashSet;
use std::sync::Arc;
use strum::{Display, EnumIter, EnumString};

mod consts;
pub mod geometry_data;
pub mod render_group;
pub mod render_modifier;
pub mod render_style;
pub mod style_id;
pub mod fps;
pub mod r_api_messenger;
pub mod worker_handler;
pub mod collision_handler;

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
            single_instance_step: true,
            ..Default::default()
        },
        WorldShapeFeatureLayerTag {
            name: "puck",
            single_instance_step: true,
            ..Default::default()
        },
    ]
}

#[derive(Default)]
pub struct WorldShapeFeatureLayerTag {
    pub name: &'static str,
    pub vertex_shader: Option<&'static str>,
    pub indirect: bool,
    pub single_instance_step: bool,
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

pub trait Renderer {
    type RAPI: RendererApi + 'static; 
    type OUTPUT;

    type INPUT<'a>;
    fn screen_size(&self) -> (f32, f32);
    fn resize(&mut self, width: u32, height: u32);
    fn update(&mut self, data: RendererUpdateData);
    fn clip_to_world(&self, coord: &Coord) -> Option<DVec2>;
    fn render(&mut self, input: Self::INPUT<'_>) -> Option<Self::OUTPUT>;

    fn api(&self) -> Arc<Self::RAPI>;

    fn clip_to_world_at_ground(
        clip_coords: &DVec2,
        inverted_view_proj: &DMat4,
    ) -> Option<DVec2> {
        let near_world = Self::clip_to_world_internal(
            &clip_coords.extend(0.0),
            inverted_view_proj,
        );

        let far_world = Self::clip_to_world_internal(
            &clip_coords.extend(1.0),
            inverted_view_proj,
        );

        let mut u = -near_world.z / (far_world.z - near_world.z);

        // let's use infinity now but in real world we have to limit it somehow
        // if u < 0.0 { return None };
        if u < 0.0 {
            u = 1.0 - u;
        }
        let result = near_world + u * (far_world - near_world);
        Some(result.truncate())
    }

    fn clip_to_world_internal(
        window: &DVec3,
        inverted_view_proj: &DMat4,
    ) -> DVec3 {
        let ndc = window.extend(1.0);
        let unprojected = inverted_view_proj * ndc;
        unprojected.truncate() / unprojected.w
    }
}

pub trait CanvasApi {
    fn set_feature_layer_tag(&mut self, tag: Option<String>);

    fn geometry_data(&mut self, geometry_data: GeometryData);
}

pub trait RendererApi: Send + Sync {
    type CANVAS: CanvasApi;
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
pub static mut SSAO_ENABLED: bool = false;
pub static mut PREVIEW_TYPE: PreviewType = PreviewType::None;

#[macro_export] macro_rules! min_f64 {
    ($x:expr) => ($x);
    ($x:expr, $($y:expr),+) => {
        ($x).min($crate::min_f64!($($y),+))
    };
}

#[macro_export] macro_rules! max_f64 {
    ($x:expr) => ($x);
    ($x:expr, $($y:expr),+) => {
        ($x).max($crate::max_f64!($($y),+))
    };
}