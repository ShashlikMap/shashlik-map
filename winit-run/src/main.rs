use map::ShashlikMap;
use map::feature_processor::ShashlikFeatureProcessor;
use map::tiles::shashlik_tiles_provider_v0::ShashlikTilesProviderV0;
use osm::source::reqwest_source::ReqwestSource;
use slint::private_unstable_api::re_exports::PointerEventKind;
use slint::wgpu_28::{WGPUConfiguration, WGPUSettings};
use slint::{GraphicsAPI, RenderingState};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc;
use wgpu::{
    Device, Limits, Queue, SurfaceConfiguration, SurfaceError, SurfaceTexture, Texture,
    TextureFormat, TextureUsages,
};
use wgpu_canvas::wgpu_canvas::WgpuCanvas;
use winit_run::App;

slint::include_modules!();

struct SlintWgpuCanvas(Queue, Device, SurfaceConfiguration, Texture);

enum SlintMapEvent {
    Pan(f32, f32),
    Pinch(f32, f32, f32),
    VerticalScroll(f32),
}

impl WgpuCanvas for SlintWgpuCanvas {
    fn queue(&self) -> &Queue {
        &self.0
    }

    fn config(&self) -> &SurfaceConfiguration {
        &self.2
    }

    fn device(&self) -> &Device {
        &self.1
    }

    fn get_current_texture(&self) -> Result<SurfaceTexture, SurfaceError> {
        todo!()
    }

    fn get_current_texture2(&self) -> &Texture {
        &self.3
    }

    fn on_resize(&mut self) {}

    fn on_pre_render(&self) {}

    fn on_post_render(&self) {}
}
fn main() {
    env_logger::init();

    let (sender, receiver) = mpsc::channel();

    let (slint_map_event_sender, slint_map_event_receiver) = mpsc::channel();

    let pointer_pos = Rc::new(Cell::new((0f32, 0f32)));
    let pointer_pos2 = Rc::clone(&pointer_pos);
    let slint_map_event_sender_internal = slint_map_event_sender.clone();
    let app = App::new(receiver, move |delta| {
        let pp = pointer_pos2.get();
        slint_map_event_sender_internal
            .send(SlintMapEvent::Pinch(delta * 100.0, pp.0, pp.1))
            .unwrap();
    });

    let mut wgpu_settings = WGPUSettings::default();
    wgpu_settings.device_required_limits = Limits::downlevel_defaults();

    slint::BackendSelector::new()
        .require_wgpu_28(WGPUConfiguration::Automatic(wgpu_settings))
        .with_winit_custom_application_handler(app)
        .select()
        .expect("Unable to create Slint backend with WGPU based renderer");

    let ui = AppKiol::new().unwrap();
    let ui_weak = ui.as_weak();
    let mut shashlik_map = None;

    ui.window()
        .set_rendering_notifier(move |state, graphics_api: &GraphicsAPI| {
            let mut last_pointer_pos = (0f32, 0f32);
            let mut pressed = false;
            match state {
                RenderingState::RenderingSetup => match graphics_api {
                    GraphicsAPI::WGPU28 {
                        instance,
                        device,
                        queue,
                        ..
                    } => {
                        let ttt = device.create_texture(&wgpu::TextureDescriptor {
                            label: None,
                            size: wgpu::Extent3d {
                                width: 1600,
                                height: 1200,
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                                | wgpu::TextureUsages::TEXTURE_BINDING,
                            view_formats: &[],
                        });
                        let config = SurfaceConfiguration {
                            usage: TextureUsages::RENDER_ATTACHMENT,
                            format: TextureFormat::Rgba8UnormSrgb,
                            width: 1600,
                            height: 1200,
                            present_mode: Default::default(),
                            desired_maximum_frame_latency: 0,
                            alpha_mode: Default::default(),
                            view_formats: vec![],
                        };
                        let canvas = SlintWgpuCanvas(queue.clone(), device.clone(), config, ttt);
                        let tiles_provider = ShashlikTilesProviderV0::new(
                            ReqwestSource::new(),
                            ShashlikFeatureProcessor::new(),
                            1.0,
                        );
                        let hh =
                            pollster::block_on(ShashlikMap::new(Box::new(canvas), tiles_provider))
                                .unwrap();

                        if let Some(ui_weak) = ui_weak.upgrade() {
                            let slint_map_event_sender_internal = slint_map_event_sender.clone();
                            let pointer_pos = Rc::clone(&pointer_pos);
                            ui_weak.on_pointer_event(move |event, x, y| match event.kind {
                                PointerEventKind::Cancel => pressed = false,
                                PointerEventKind::Down => pressed = true,
                                PointerEventKind::Up => pressed = false,
                                PointerEventKind::Move => {
                                    if pressed {
                                        let delta_x = -(x - last_pointer_pos.0) / 10.0;
                                        let delta_y = -(y - last_pointer_pos.1) / 10.0;
                                        slint_map_event_sender_internal
                                            .send(SlintMapEvent::Pan(delta_x, delta_y))
                                            .unwrap();
                                    }
                                    pointer_pos.set((x, y));
                                    last_pointer_pos = (x, y);
                                }
                                _ => {}
                            });
                            let slint_map_event_sender_internal = slint_map_event_sender.clone();
                            ui_weak.on_vert_scroll(move |delta_y| {
                                slint_map_event_sender_internal
                                    .send(SlintMapEvent::VerticalScroll(delta_y / 5.0))
                                    .unwrap();
                            });
                        }

                        shashlik_map = Some(hh);

                        shashlik_map.as_mut().unwrap().resize(1600, 1200);
                        shashlik_map.as_mut().unwrap().set_camera_follow_mode(false);
                    }
                    _ => {}
                },
                RenderingState::BeforeRendering => {
                    if let (Some(shashlik_map), Some(app)) =
                        (shashlik_map.as_mut(), ui_weak.upgrade())
                    {
                        if let Ok(event) = slint_map_event_receiver.try_recv() {
                            match event {
                                SlintMapEvent::Pan(dx, dy) => {
                                    shashlik_map.pan_delta(dx, dy);
                                }
                                SlintMapEvent::VerticalScroll(delta_y) => {
                                    shashlik_map.pitch_delta(delta_y);
                                }
                                SlintMapEvent::Pinch(delta, x, y) => {
                                    shashlik_map.zoom_delta(delta, (x, y));
                                }
                            };
                        }
                        let target_texture = shashlik_map.update_and_render();
                        app.set_texture(slint::Image::try_from(target_texture).unwrap());
                        app.window().request_redraw();
                    }
                }
                RenderingState::AfterRendering => {}
                RenderingState::RenderingTeardown => {}
                _ => panic!("Unhandled RenderingState "),
            }
        })
        .expect("Can't set Slint rendering_notifier");

    // let sender_clone = sender.clone();
    // ui.on_open_kml_button_click(move || {
    //     let path = DialogBuilder::file()
    //         .set_location("~/Desktop")
    //         .add_filter("KML", ["kml"])
    //         .open_single_file()
    //         .show()
    //         .unwrap();
    //     if let Some(path) = path {
    //         sender_clone.send(CustomUIEvent::KMLPath(path)).unwrap();
    //     }
    // });

    ui.run().unwrap();
}
