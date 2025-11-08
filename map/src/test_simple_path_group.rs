use lyon::path::Path;
use renderer::canvas_api::CanvasApi;
use renderer::geometry_data::ShapeData;
use renderer::draw_commands::{GeometryType, PolylineOptions};
use renderer::render_group::RenderGroup;
use renderer::styles::style_id::StyleId;

pub struct TestSimplePathGroup {
    pub path: Path,
    pub style_id: StyleId,
}

impl RenderGroup for TestSimplePathGroup {
    fn content(&self, canvas: &mut CanvasApi) {
        let options = PolylineOptions {
            width: 0.7,
        };

        canvas.path(
            &ShapeData {
                path: self.path.clone(),
                geometry_type: GeometryType::Polyline(options),
                style_id: self.style_id.clone(),
                layer_level: 0,
                is_screen: true,
            },
        );
    }
}
