use map::feature_processor::ShashlikFeatureProcessor;
use map::route::RouteCosting;
use map::tiles::shashlik_tiles_provider_v0::ShashlikTilesProviderV0;
use map::ShashlikMap;
use native_dialog::DialogBuilder;
use osm::source::reqwest_source::ReqwestSource;
use slint::private_unstable_api::re_exports::PointerEventKind;
use slint::wgpu_28::{WGPUConfiguration, WGPUSettings};
use slint::{GraphicsAPI, PhysicalSize, RenderingState};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc;
use wgpu::SurfaceConfiguration;
use wgpu::TextureFormat;
use wgpu::TextureUsages;
use wgpu::{Features, Limits};
use wgpu_canvas::wgpu_canvas::DefaultWgpuCanvas;
use wgpu_canvas::{PREVIEW_ENABLED, SHADOWS_ENABLED, SSAO_ENABLED};

slint::include_modules!();

enum SlintMapEvent {
    Pan(f32, f32),
    Pinch(f32, f32, f32),
    VerticalScroll(f32),
    FollowMode(bool),
    FeatureEnabled(Feature, bool),
    BtnAction(Action, i32),
}

fn main() {
    env_logger::init();

    let (slint_map_event_sender, slint_map_event_receiver) = mpsc::channel();

    let pointer_pos = Rc::new(Cell::new((0f32, 0f32)));

    let mut wgpu_settings = WGPUSettings::default();
    wgpu_settings.device_required_features =
        Features::VERTEX_WRITABLE_STORAGE | Features::CLEAR_TEXTURE;
    wgpu_settings.device_required_limits = Limits::downlevel_defaults();

    slint::BackendSelector::new()
        .require_wgpu_28(WGPUConfiguration::Automatic(wgpu_settings))
        .select()
        .expect("Unable to create Slint backend with WGPU based renderer");

    let ui = ShashlikUI::new().unwrap();
    let mut screen_size = ui.window().size();
    println!("screen size: {:?}", screen_size);
    if screen_size.width == 0 || screen_size.height == 0 {
        screen_size = PhysicalSize::new(2000, 1200);
    }
    ui.set_screen_width(screen_size.width as i32);
    ui.set_screen_height(screen_size.height as i32);

    let dpi = if screen_size.height <= 600 { 0.7 } else { 1.0 };
    let texture_width = ui.get_requested_texture_width();
    let texture_height = ui.get_requested_texture_height();
    println!(
        "texture width: {} and height: {}",
        texture_width, texture_height
    );
    let scale_factor = ui.window().scale_factor();
    println!("scale_factor: {}", scale_factor);
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
                                width: texture_width as u32,
                                height: texture_height as u32,
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
                            width: texture_width as u32,
                            height: texture_height as u32,
                            present_mode: Default::default(),
                            desired_maximum_frame_latency: 0,
                            alpha_mode: Default::default(),
                            view_formats: vec![],
                        };
                        let canvas = DefaultWgpuCanvas(
                            queue.clone(),
                            device.clone(),
                            config,
                            target_texture,
                        );
                        let tiles_provider = ShashlikTilesProviderV0::new(
                            ReqwestSource::new(),
                            ShashlikFeatureProcessor::new(),
                            dpi,
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
                            ui_weak.on_pinch(move |delta, x, y| {
                                slint_map_event_sender_internal
                                    .send(SlintMapEvent::Pinch(-delta * 1.0, x, y))
                                    .unwrap();
                            });

                            let slint_map_event_sender_internal = slint_map_event_sender.clone();
                            ui_weak.on_follow_mode(move |enabled| {
                                slint_map_event_sender_internal
                                    .send(SlintMapEvent::FollowMode(enabled))
                                    .unwrap();
                            });

                            let slint_map_event_sender_internal = slint_map_event_sender.clone();
                            ui_weak.on_feature_enabled(move |feature, enabled| {
                                slint_map_event_sender_internal
                                    .send(SlintMapEvent::FeatureEnabled(feature, enabled))
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
                        map.resize(texture_width as u32, texture_height as u32);
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
                                            _ => panic!("{cost_index} cost index not supported"),
                                        };
                                        shashlik_map
                                            .create_route_to_from_screen_center(route_costing);
                                    }
                                    Action::KML => {
                                        let path = DialogBuilder::file()
                                            .set_location("~/Desktop")
                                            .add_filter("KML", ["kml"])
                                            .open_single_file()
                                            .show()
                                            .unwrap();
                                        if let Some(path) = path {
                                            shashlik_map.load_kml_path(path)
                                        }
                                    }
                                },
                                SlintMapEvent::FeatureEnabled(feature, enabled) => match feature {
                                    Feature::SSAO => unsafe {
                                        SSAO_ENABLED = enabled;
                                    },
                                    Feature::Shadows => unsafe {
                                        SHADOWS_ENABLED = enabled;
                                    },
                                    Feature::Preview => unsafe {
                                        PREVIEW_ENABLED = enabled;
                                    },
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
