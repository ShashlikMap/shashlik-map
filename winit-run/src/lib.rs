use i_slint_backend_winit::{CustomApplicationHandler, EventResult};
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

pub struct PinchWorkaroundHandler {
    pub pinch_cb: Box<dyn FnMut(f32)>,
}

// pub enum CustomUIEvent {
//     KMLPath(PathBuf),
// }

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
                // drop(self.shashlik_map.take());
                event_loop.exit();
            }
            WindowEvent::PinchGesture { delta, .. } => {
                (self.pinch_cb)(*delta as f32);
            }
            // WindowEvent::KeyboardInput {
            //     event:
            //         KeyEvent {
            //             physical_key: PhysicalKey::Code(code),
            //             state: key_state,
            //             ..
            //         },
            //     ..
            // } => {
            //     let is_pressed = key_state.is_pressed();
            //     if *code == KeyCode::Escape && is_pressed {
            //         event_loop.exit();
            //     } else {
            //         match code {
            //             KeyCode::KeyN => {
            //                 if is_pressed {
            //                     // map.set_camera_follow_mode(!map.get_camera_follow_mode());
            //                 }
            //             }
            //             KeyCode::KeyB => {
            //                 if is_pressed {
            //                     // RouteCosting::Motorbike for winit by default
            //                     // map.create_route_to_from_screen_center(RouteCosting::Motorbike);
            //                 }
            //             }
            //             KeyCode::KeyM => {
            //                 if is_pressed {
            //                     self.fake_bearing += 30.0;
            //                     //DM office 139.74777078320227 35.62298925839326
            //                     //Ugusuidani office 139.7769298 35.7248164
            //
            //                     // map.set_lon_lat_bearing( 139.74777078320227, 35.62298925839326, Some(self.fake_bearing));
            //
            //                 }
            //             }
            //             _ => {}
            //         }
            //     }
            // }
            _ => {}
        }
        EventResult::Propagate
    }
}
