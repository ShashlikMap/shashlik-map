use renderer_common::render_modifier::SpatialData;
use crate::vertex_attrs::{GeneralInstanceInput, ShapeInstanceInput};
use bytemuck::Pod;
use glam::DVec3;

pub trait MeshInstanceInput: Sized + Pod {
    fn fill_attrs(
        attrs: &mut Vec<Self>,
        cs_offset: &DVec3,
        original_positions_alpha: &Vec<(DVec3, f32)>,
        spatial_data: &SpatialData,
        double_style: bool,
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
            let instance_input = Self::create_instance_struct(
                transform_with_cs_offset.as_vec3().to_array(),
                item.1,
                matrix.as_mat4().to_cols_array_2d(),
                [
                    bbox_origin_with_cs_offset.x as f32,
                    bbox_origin_with_cs_offset.y as f32,
                    spatial_data.bbox.width() as f32,
                    spatial_data.bbox.height() as f32,
                ],
                spatial_data.normal_scale as f32
            );
            attrs.push(instance_input);
            if double_style {
                attrs.push(instance_input);
            }
        }
    }

    fn create_instance_struct(
        position: [f32; 3],
        color_alpha: f32,
        matrix: [[f32; 4]; 4],
        bbox: [f32; 4],
        normal_scale: f32
    ) -> Self;
}

impl MeshInstanceInput for GeneralInstanceInput {
    fn create_instance_struct(
        position: [f32; 3],
        color_alpha: f32,
        matrix: [[f32; 4]; 4],
        _bbox: [f32; 4],
        _normal_scale: f32
    ) -> Self {
        GeneralInstanceInput {
            position,
            color_alpha,
            matrix,
        }
    }
}

impl MeshInstanceInput for ShapeInstanceInput {
    fn create_instance_struct(
        position: [f32; 3],
        color_alpha: f32,
        matrix: [[f32; 4]; 4],
        bbox: [f32; 4],
        normal_scale: f32
    ) -> Self {
        ShapeInstanceInput {
            position,
            color_alpha,
            matrix,
            bbox,
            normal_scale,
            _padding: [0; 3]
        }
    }
}