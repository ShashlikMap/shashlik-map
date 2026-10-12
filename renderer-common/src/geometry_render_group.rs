use crate::CanvasApi;
use crate::geometry_data::GeometryData;
use crate::render_group::RenderGroup;

pub struct GeometryRenderGroup(pub GeometryData);

impl<T: CanvasApi> RenderGroup<T> for GeometryRenderGroup {
    fn content(self: Box<Self>, canvas: &mut T) {
        canvas.geometry_data(self.0);
    }
}
