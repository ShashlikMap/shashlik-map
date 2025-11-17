use crate::canvas_api::CanvasApi;

pub trait RenderGroup: Send {
    fn content(&mut self, canvas: &mut CanvasApi);
}
