use renderer::canvas_api::CanvasApi;
use crate::mesh_loader::MeshLoader;
use renderer::geometry_data::ShapeData;
use renderer::draw_commands::GeometryType;
use renderer::render_group::RenderGroup;
use renderer::styles::style_id::StyleId;

pub struct SimplePuck {}

impl RenderGroup for SimplePuck {
    fn content(&mut self, canvas: &mut CanvasApi) {
        canvas.set_feature_layer_tag(Some("puck_layer".to_string()));
        canvas.path(
            ShapeData {
                path: MeshLoader::load_simple_puck(),
                geometry_type: GeometryType::Polygon,
                style_id: StyleId("puck_style"),
                index_layer_level: 0,
                is_screen: false,
            },
        );
    }
}
