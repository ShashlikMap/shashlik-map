use crate::mesh_loader::MeshLoader;
use renderer_common::geometry_data::{GeometryData, GeometryType, ShapeData, StyledRangeInfo};
use renderer_common::render_group::RenderGroup;
use renderer_common::style_id::StyleId;
use renderer_common::CanvasApi;

pub struct SimplePuck {}

impl <T: CanvasApi> RenderGroup<T> for SimplePuck {
    fn content(&mut self, canvas: &mut T) {
        canvas.set_feature_layer_tag(Some("puck".to_string()));
        canvas.geometry_data(GeometryData::Shape(
            ShapeData {
                path: MeshLoader::load_simple_puck(),
                geometry_type: GeometryType::Polygon,
                style_id: StyleId::new("puck_style"),
                index_layer_level: 0,
                styled_range_info: StyledRangeInfo(1, "")
            },
        ));
    }
}
