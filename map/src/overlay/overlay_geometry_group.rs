use renderer_common::CanvasApi;
use renderer_common::geometry_data::GeometryData;
use renderer_common::render_group::RenderGroup;

pub(crate) struct OverlayGeometryGroup(GeometryData);

impl<T: CanvasApi> RenderGroup<T> for OverlayGeometryGroup {
    fn content(self: Box<Self>, canvas: &mut T) {
        canvas.geometry_data(self.0);
    }
}
