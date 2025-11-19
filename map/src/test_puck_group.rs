use renderer::canvas_api::CanvasApi;
use crate::tiles::mesh_loader::MeshLoader;
use renderer::geometry_data::ShapeData;
use renderer::draw_commands::GeometryType;
use renderer::render_group::RenderGroup;
use renderer::styles::style_id::StyleId;

pub struct TestSimplePuck {}

impl RenderGroup for TestSimplePuck {
    fn content(&mut self, canvas: &mut CanvasApi) {
        canvas.path(
            ShapeData {
                path: MeshLoader::load_simple_puck(),
                geometry_type: GeometryType::Polygon,
                style_id: StyleId("puck_style"),
                index_layer_level: 0,
                is_screen: true,
            },
        );
    }
}
