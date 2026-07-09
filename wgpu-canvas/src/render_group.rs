use crate::CanvasApi;

pub trait RenderGroup<T: CanvasApi>: Send {
    fn content(&mut self, canvas: &mut T);
}
