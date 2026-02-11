use crate::modifier::render_modifier::SpatialData;
use crate::vertex_attrs::{GeneralInstanceInput, ShapeInstanceInput};
use bytemuck::Pod;
use cgmath::Vector3;

pub trait MeshInstanceInput: Sized + Pod {
    fn fill_attrs(
        attrs: &mut Vec<Self>,
        cs_offset: &Vector3<f64>,
        original_positions_alpha: &Vec<(Vector3<f64>, f32)>,
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
            let instance_input = Self::create_instance_struct(
                transform_with_cs_offset.cast().unwrap().into(),
                item.1,
                matrix.cast().unwrap().into(),
                [
                    transform_with_cs_offset.x as f32,
                    transform_with_cs_offset.y as f32,
                    spatial_data.size.0.round() as f32,
                    spatial_data.size.1.round() as f32,
                ],
                spatial_data.normal_scale as f32,
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
        normal_scale: f32,
    ) -> Self;
}

impl MeshInstanceInput for GeneralInstanceInput {
    fn create_instance_struct(
        position: [f32; 3],
        color_alpha: f32,
        matrix: [[f32; 4]; 4],
        _bbox: [f32; 4],
        _normal_scale: f32,
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
        normal_scale: f32,
    ) -> Self {
        ShapeInstanceInput {
            position,
            color_alpha,
            matrix,
            bbox,
            normal_scale,
        }
    }
}