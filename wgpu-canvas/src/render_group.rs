use crate::MyCanvasApi;

pub trait RenderGroup<T: MyCanvasApi>: Send {
    fn content(&mut self, canvas: &mut T);
}
