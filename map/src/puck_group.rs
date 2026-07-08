use renderer::canvas_api::CanvasApi;
use crate::mesh_loader::MeshLoader;
use renderer::geometry_data::{GeometryData, ShapeData};
use renderer::draw_commands::GeometryType;
use renderer::mesh::mesh::StyledRangeInfo;
use wgpu_canvas::MyCanvasApi;
use wgpu_canvas::render_group::RenderGroup;
use wgpu_canvas::style_id::StyleId;

pub struct SimplePuck {}

impl <T: MyCanvasApi> RenderGroup<T> for SimplePuck {
    fn content(&mut self, canvas: &mut T) {
        canvas.set_feature_layer_tag(Some("puck_layer".to_string()));
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
