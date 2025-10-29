use lyon::path::Path;
use renderer::canvas_api::CanvasApi;
use renderer::geometry_data::ShapeData;
use renderer::draw_commands::GeometryType;
use renderer::render_group::RenderGroup;
use renderer::styles::style_id::StyleId;

pub struct TestSimplePathGroup {
    pub path: Path,
    pub style_id: StyleId,
}

impl RenderGroup for TestSimplePathGroup {
    fn content(&self, canvas: &mut CanvasApi) {
        canvas.path(
            &ShapeData {
                path: self.path.clone(),
                geometry_type: GeometryType::Polyline,
                style_id: self.style_id.clone(),
            },
            true,
        );
    }
}
