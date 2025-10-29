use crate::canvas_api::CanvasApi;

pub trait RenderGroup: Send {
    fn content(&self, canvas: &mut CanvasApi);
}
