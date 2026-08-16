use crate::vertex_attrs::{GeneralInstanceInput, ScreenShapeInstanceInput, ShapeInstanceInput};
use bytemuck::Pod;
use glam::DVec3;
use renderer_common::render_modifier::SpatialData;
use crate::mesh_layers::{LayerAttrMapper, LayerAttribute};

pub trait MeshInstanceInput: Sized + Pod + From<LayerAttribute> {
    fn fill_attrs(
        attrs: &mut Vec<Self>,
        attr_mapper: LayerAttrMapper<Self>,
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
                + DVec3::new(spatial_data.bbox.min().x, spatial_data.bbox.min().y, 0.0)
                - cs_offset;

            let instance_input = attr_mapper(LayerAttribute {
                position: transform_with_cs_offset.as_vec3().to_array(),
                color_alpha: item.1,
                matrix: matrix.as_mat4().to_cols_array_2d(),
                bbox: [
                    bbox_origin_with_cs_offset.x as f32,
                    bbox_origin_with_cs_offset.y as f32,
                    spatial_data.bbox.width() as f32,
                    spatial_data.bbox.height() as f32,
                ],
                normal_scale: spatial_data.normal_scale as f32,
                ..Default::default()
            });

            attrs.push(instance_input);
        }
    }
}

impl<T> MeshInstanceInput for T where T: Clone + Default + Sized + Pod + From<LayerAttribute> {}

impl From<LayerAttribute> for GeneralInstanceInput {
    fn from(value: LayerAttribute) -> Self {
        GeneralInstanceInput {
            position: value.position,
            color_alpha: value.color_alpha,
            matrix: value.matrix,
        }
    }
}

impl From<LayerAttribute> for ShapeInstanceInput {
    fn from(value: LayerAttribute) -> Self {
        ShapeInstanceInput {
            position: value.position,
            color_alpha: value.color_alpha,
            matrix: value.matrix,
            bbox: value.bbox,
            normal_scale: value.normal_scale,
            _padding: [0; 3],
        }
    }
}

impl From<LayerAttribute> for ScreenShapeInstanceInput {
    fn from(value: LayerAttribute) -> Self {
        ScreenShapeInstanceInput {
            position: value.position,
            color_alpha: value.color_alpha,
            matrix: value.matrix,
            screen_space: value.screen_space,
        }
    }
}
