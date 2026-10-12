use crate::CanvasApi;

pub trait RenderGroup<T: CanvasApi>: Send {
    fn content(self: Box<Self>, canvas: &mut T);
}
