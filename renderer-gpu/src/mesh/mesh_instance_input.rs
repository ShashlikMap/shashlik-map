use crate::vertex_attrs::{GeneralInstanceInput, ScreenShapeInstanceInput, ShapeInstanceInput};
use bytemuck::Pod;
use glam::DVec3;
use renderer_common::render_modifier::SpatialData;

#[derive(Default)]
pub struct MeshInstanceInputBuilder {
    position: [f32; 3],
    color_alpha: f32,
    matrix: [[f32; 4]; 4],
    bbox: Option<[f32; 4]>,
    normal_scale: Option<f32>,
    screen_space: Option<u32>,
}

impl MeshInstanceInputBuilder {

    pub fn with_bbox(mut self, bbox: [f32; 4]) -> Self {
        self.bbox = Some(bbox);
        self
    }

    pub fn with_normal_scale(mut self, normal_scale: f32) -> Self {
        self.normal_scale = Some(normal_scale);
        self
    }

    pub fn with_screen_space(mut self, screen_space: u32) -> Self {
        self.screen_space = Some(screen_space);
        self
    }

    fn position(&self) -> [f32; 3] {
        self.position
    }

    fn color_alpha(&self) -> f32 {
        self.color_alpha
    }

    fn matrix(&self) -> [[f32; 4]; 4] {
        self.matrix
    }

    fn bbox(&self) -> [f32; 4] {
        self.bbox.expect("MeshInstanceInputBuilder expected bbox")
    }

    fn normal_scale(&self) -> f32 {
        self.normal_scale.expect("MeshInstanceInputBuilder expected normal_scale")
    }

    fn screen_space(&self) -> u32 {
        self.screen_space.expect("MeshInstanceInputBuilder expected screen_space")
    }
    pub(crate) fn build<I: MeshInstanceInput>(self) -> I {
        I::create_instance_struct(self)
    }
}

pub trait MeshInstanceInput: Sized + Pod {
    fn builder(position: [f32; 3],
               color_alpha: f32,
               matrix: [[f32; 4]; 4]) -> MeshInstanceInputBuilder {
        MeshInstanceInputBuilder {
            position,
            color_alpha,
            matrix,
            ..Default::default()
        }
    }

    fn fill_attrs(
        attrs: &mut Vec<Self>,
        cs_offset: &DVec3,
        original_positions_alpha: &Vec<(DVec3, f32)>,
        spatial_data: &SpatialData,
    ) {
        attrs.clear();
        let matrix = spatial_data.scale_rot_matrix();
        for i in 0..original_positions_alpha.len() {
            let item = original_positions_alpha[i];
            if item.1 <= 0.0 {
                continue;
            }

            let transform_with_cs_offset = item.0 + spatial_data.transform - cs_offset;

            let bbox_origin_with_cs_offset = item.0
                + DVec3::new(spatial_data.bbox.min().x, spatial_data.bbox.min().y, 0.0) - cs_offset;
            let instance_input = Self::builder(transform_with_cs_offset.as_vec3().to_array(),
                                               item.1,
                                               matrix.as_mat4().to_cols_array_2d(),
            ).with_bbox([
                bbox_origin_with_cs_offset.x as f32,
                bbox_origin_with_cs_offset.y as f32,
                spatial_data.bbox.width() as f32,
                spatial_data.bbox.height() as f32,
            ])
                .with_normal_scale(spatial_data.normal_scale as f32).build();
            attrs.push(instance_input);
        }
    }

    fn create_instance_struct(builder: MeshInstanceInputBuilder) -> Self;
}

impl MeshInstanceInput for GeneralInstanceInput {
    fn create_instance_struct(builder: MeshInstanceInputBuilder) -> Self {
        Self {
            position: builder.position(),
            color_alpha: builder.color_alpha(),
            matrix: builder.matrix(),
        }
    }
}

impl MeshInstanceInput for ShapeInstanceInput {

    fn create_instance_struct(builder: MeshInstanceInputBuilder) -> Self {
        Self {
            position: builder.position(),
            color_alpha: builder.color_alpha(),
            matrix: builder.matrix(),
            bbox: builder.bbox(),
            normal_scale: builder.normal_scale(),
            _padding: [0; 3],
        }
    }
}

impl MeshInstanceInput for ScreenShapeInstanceInput {
    fn create_instance_struct(builder: MeshInstanceInputBuilder) -> Self {
        Self {
            position: builder.position(),
            color_alpha: builder.color_alpha(),
            matrix: builder.matrix(),
            screen_space: builder.screen_space(),
        }
    }
}