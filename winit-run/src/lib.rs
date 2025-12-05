use app_surface::{AppSurface, SurfaceFrame};
use i_slint_backend_winit::{CustomApplicationHandler, EventResult};
use map::tiles::tiles_provider::TilesProvider;
use map::ShashlikMap;
use renderer::camera::CameraController;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use wgpu::{Device, Queue, SurfaceConfiguration, SurfaceError, SurfaceTexture};
use winit::dpi::PhysicalPosition;
use wgpu_canvas::wgpu_canvas::WgpuCanvas;
use winit::event::{KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

pub struct App<T: TilesProvider> {
    pub camera_controller: Rc<RefCell<CameraController>>,
    pub receiver: Receiver<CustomUIEvent>,
    pub get_tiles_provider: Box<dyn Fn() -> T>,
    pub shashlik_map: Option<ShashlikMap<T>>,
    pub cursor_active: bool,
    pub last_cursor_position: PhysicalPosition<f64>,
    pub fake_bearing: f32
}

pub enum CustomUIEvent {
    KMLPath(PathBuf),
}

impl<T: TilesProvider> App<T> {
    pub fn new(get_tiles_provider: Box<dyn Fn() -> T>, receiver: Receiver<CustomUIEvent>) -> Self {
        Self {
            camera_controller: Rc::new(RefCell::new(CameraController::new(1.0))),
            receiver,
            get_tiles_provider,
            shashlik_map: None,
            cursor_active: false,
            last_cursor_position: PhysicalPosition::new(0.0, 0.0),
            fake_bearing: 0.0
        }
    }
}

pub struct WinitAppSurface {
    pub app_surface: AppSurface,
}
impl WgpuCanvas for WinitAppSurface {
    fn queue(&self) -> &Queue {
        &self.app_surface.queue
    }

    fn config(&self) -> &SurfaceConfiguration {
        &self.app_surface.config
    }

    fn device(&self) -> &Device {
        &self.app_surface.device
    }

    fn get_current_texture(&self) -> Result<SurfaceTexture, SurfaceError> {
        self.app_surface.surface.get_current_texture()
    }

    fn on_resize(&mut self, _width: u32, _height: u32) {
        self.app_surface.resize_surface();
    }

    fn on_pre_render(&self) {
        self.app_surface.pre_present_notify();
    }

    fn on_post_render(&self) {
        self.app_surface.request_redraw();
    }
}

impl<T: TilesProvider> CustomApplicationHandler for App<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) -> EventResult {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let app_view = futures_lite::future::block_on(AppSurface::new(window));
        let winit_surface = WinitAppSurface {
            app_surface: app_view,
        };

        let tiles_provider = (self.get_tiles_provider)();
        let wgpu_state = pollster::block_on(ShashlikMap::new_with_camera_controller(
            Rc::clone(&self.camera_controller),
            Box::new(winit_surface),
            tiles_provider
        ))
        .unwrap();
        self.shashlik_map = Some(wgpu_state);
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
        if self.shashlik_map.is_none() {
            return EventResult::Propagate;
        }
        let map = self.shashlik_map.as_mut().unwrap();

        if let Ok(event) = self.receiver.try_recv() {
            match event {
                CustomUIEvent::KMLPath(path) => {
                    map.load_kml_path(path);
                }
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                drop(self.shashlik_map.take());
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                map.renderer().resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                map.update_and_render();
            }
            WindowEvent::MouseInput { state, button, .. } => match (button, state.is_pressed()) {
                (MouseButton::Left, true) => {
                    self.cursor_active = true;
                }
                (MouseButton::Left, false) => {
                    self.cursor_active = false;
                }
                _ => {}
            },
            WindowEvent::CursorMoved { position, .. } => {
                if self.cursor_active {
                    let delta_x = -(position.x - self.last_cursor_position.x) / 10.0;
                    let delta_y = -(position.y - self.last_cursor_position.y) / 10.0;
                    self.shashlik_map.as_ref().unwrap().pan_delta(delta_x as f32, delta_y as f32)
                }
                self.last_cursor_position = position.clone();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                match delta {
                    MouseScrollDelta::LineDelta(_, _) => {}
                    MouseScrollDelta::PixelDelta(delta_xy) => {
                        self.shashlik_map.as_ref().unwrap().zoom_delta((delta_xy.y/10.0) as f32, self.last_cursor_position.cast::<f32>().into());
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => {
                let is_pressed = key_state.is_pressed();
                if *code == KeyCode::Escape && is_pressed {
                    event_loop.exit();
                } else {
                    match code {
                        KeyCode::KeyN => {
                            if is_pressed {
                                map.cam_follow_mode = !map.cam_follow_mode;
                            }
                        }
                        KeyCode::KeyM => {
                            if is_pressed {
                                self.fake_bearing += 30.0;
                                map.set_lat_lon_bearing(35.7248164, 139.7769298, Some(self.fake_bearing));
                            }
                        }
                        _ => {}
                    }
                }
            }

            _ => {}
        }
        EventResult::Propagate
    }
}
