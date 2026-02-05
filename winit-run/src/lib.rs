use i_slint_backend_winit::{CustomApplicationHandler, EventResult};
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

pub struct PinchWorkaroundHandler {
    pub pinch_cb: Box<dyn FnMut(f32)>,
}

impl PinchWorkaroundHandler {
    pub fn new<F: FnMut(f32) + 'static>(pinch_cb: F) -> Self {
        Self {
            pinch_cb: Box::new(pinch_cb),
        }
    }
}
impl CustomApplicationHandler for PinchWorkaroundHandler {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) -> EventResult {
        EventResult::Propagate
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _winit_window: Option<&Window>,
        _slint_window: Option<&slint::Window>,
        event: &WindowEvent,
    ) -> EventResult {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::PinchGesture { delta, .. } => {
                (self.pinch_cb)(*delta as f32);
            }
            _ => {}
        }
        EventResult::Propagate
    }
}
