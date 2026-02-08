use crate::canvas::SlintWgpuCanvas;
use map::ShashlikMap;
use map::feature_processor::ShashlikFeatureProcessor;
use map::route::RouteCosting;
use map::tiles::shashlik_tiles_provider_v0::ShashlikTilesProviderV0;
use osm::source::reqwest_source::ReqwestSource;
use slint::private_unstable_api::re_exports::PointerEventKind;
use slint::wgpu_28::{WGPUConfiguration, WGPUSettings};
use slint::{GraphicsAPI, RenderingState};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc;
use wgpu::Limits;
use wgpu::SurfaceConfiguration;
use wgpu::TextureFormat;
use wgpu::TextureUsages;
use winit_run::PinchWorkaroundHandler;

pub(crate) mod canvas;

slint::include_modules!();

enum SlintMapEvent {
    Pan(f32, f32),
    Pinch(f32, f32, f32),
    VerticalScroll(f32),
    FollowMode(bool),
    BtnAction(Action, i32),
}

fn main() {
    // TODO Correct UI resize
    const SCREEN_WIDTH: u32 = 1600;
    const SCREEN_HEIGHT: u32 = 1200;

    env_logger::init();

    let (slint_map_event_sender, slint_map_event_receiver) = mpsc::channel();

    let pointer_pos = Rc::new(Cell::new((0f32, 0f32)));
    let pointer_pos_internal = Rc::clone(&pointer_pos);
    let slint_map_event_sender_internal = slint_map_event_sender.clone();
    let app = PinchWorkaroundHandler::new(move |delta| {
        let (x, y) = pointer_pos_internal.get();
        slint_map_event_sender_internal
            .send(SlintMapEvent::Pinch(delta * 100.0, x, y))
            .unwrap();
    });

    let mut wgpu_settings = WGPUSettings::default();
    wgpu_settings.device_required_limits = Limits::downlevel_defaults();

    slint::BackendSelector::new()
        .require_wgpu_28(WGPUConfiguration::Automatic(wgpu_settings))
        .with_winit_custom_application_handler(app)
        .select()
        .expect("Unable to create Slint backend with WGPU based renderer");

    let ui = ShashlikUI::new().unwrap();
    let ui_weak = ui.as_weak();
    let mut shashlik_map = None;

    ui.window()
        .set_rendering_notifier(move |state, graphics_api: &GraphicsAPI| {
            let mut pressed = false;
            match state {
                RenderingState::RenderingSetup => match graphics_api {
                    GraphicsAPI::WGPU28 { device, queue, .. } => {
                        let target_texture = device.create_texture(&wgpu::TextureDescriptor {
                            label: None,
                            size: wgpu::Extent3d {
                                width: SCREEN_WIDTH,
                                height: SCREEN_HEIGHT,
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            // TODO How to use Bgra?
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                                | wgpu::TextureUsages::TEXTURE_BINDING,
                            view_formats: &[],
                        });
                        let config = SurfaceConfiguration {
                            usage: TextureUsages::RENDER_ATTACHMENT,
                            format: TextureFormat::Rgba8UnormSrgb,
                            width: SCREEN_WIDTH,
                            height: SCREEN_HEIGHT,
                            present_mode: Default::default(),
                            desired_maximum_frame_latency: 0,
                            alpha_mode: Default::default(),
                            view_formats: vec![],
                        };
                        let canvas =
                            SlintWgpuCanvas(queue.clone(), device.clone(), config, target_texture);
                        let tiles_provider = ShashlikTilesProviderV0::new(
                            ReqwestSource::new(),
                            ShashlikFeatureProcessor::new(),
                            1.0,
                        );

                        if let Some(ui_weak) = ui_weak.upgrade() {
                            let slint_map_event_sender_internal = slint_map_event_sender.clone();
                            let pointer_pos = Rc::clone(&pointer_pos);
                            ui_weak.on_pointer_event(move |event, x, y| match event.kind {
                                PointerEventKind::Cancel => pressed = false,
                                PointerEventKind::Down => pressed = true,
                                PointerEventKind::Up => pressed = false,
                                PointerEventKind::Move => {
                                    if pressed {
                                        let delta_x = -(x - pointer_pos.get().0) / 10.0;
                                        let delta_y = -(y - pointer_pos.get().1) / 10.0;
                                        slint_map_event_sender_internal
                                            .send(SlintMapEvent::Pan(delta_x, delta_y))
                                            .unwrap();
                                    }
                                    pointer_pos.set((x, y));
                                }
                                _ => {}
                            });

                            // TODO How to get rid of all this clones?
                            let slint_map_event_sender_internal = slint_map_event_sender.clone();
                            ui_weak.on_vert_scroll(move |delta_y| {
                                slint_map_event_sender_internal
                                    .send(SlintMapEvent::VerticalScroll(delta_y / 5.0))
                                    .unwrap();
                            });

                            let slint_map_event_sender_internal = slint_map_event_sender.clone();
                            ui_weak.on_follow_mode(move |enabled| {
                                slint_map_event_sender_internal
                                    .send(SlintMapEvent::FollowMode(enabled))
                                    .unwrap();
                            });

                            let slint_map_event_sender_internal = slint_map_event_sender.clone();
                            ui_weak.on_btn_click(move |action, cost_index| {
                                slint_map_event_sender_internal
                                    .send(SlintMapEvent::BtnAction(action, cost_index))
                                    .unwrap()
                            });
                        }

                        let mut map =
                            pollster::block_on(ShashlikMap::new(Box::new(canvas), tiles_provider))
                                .unwrap();
                        map.resize(SCREEN_WIDTH, SCREEN_HEIGHT);
                        shashlik_map = Some(map);
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
                                SlintMapEvent::FollowMode(enabled) => {
                                    shashlik_map.set_camera_follow_mode(enabled);
                                }
                                SlintMapEvent::BtnAction(action, cost_index) => match action {
                                    Action::DmOffice => {
                                        shashlik_map.set_lon_lat_bearing(
                                            139.74777078320227,
                                            35.62298925839326,
                                            Some(0f32),
                                        );
                                    }
                                    Action::Route => {
                                        let route_costing = match cost_index {
                                            0 => RouteCosting::Pedestrian,
                                            1 => RouteCosting::Auto,
                                            2 => RouteCosting::Motorbike,
                                            _ => panic!("{cost_index} cost index not supported")
                                        };
                                        shashlik_map.create_route_to_from_screen_center(
                                            route_costing,
                                        );
                                    }
                                    Action::KML => {
                                        // TODO Fix KML loading after fixing georust KML
                                    }
                                },
                            };
                        }
                        let target_texture = shashlik_map.update_and_render();
                        app.set_texture(slint::Image::try_from(target_texture.unwrap()).unwrap());
                        app.window().request_redraw();
                    }
                }
                RenderingState::AfterRendering => {}
                RenderingState::RenderingTeardown => {}
                _ => panic!("Unhandled RenderingState "),
            }
        })
        .expect("Can't set Slint rendering_notifier");

    ui.run().unwrap();
}
