use map::feature_processor::ShashlikFeatureProcessor;
use map::route::RouteCosting;
use map::tiles::shashlik_tiles_provider_v0::ShashlikTilesProviderV0;
use map::ShashlikMap;
use native_dialog::DialogBuilder;
use osm::source::reqwest_source::ReqwestSource;
use slint::wgpu_29::{WGPUConfiguration, WGPUSettings};
use slint::{GraphicsAPI, PhysicalSize, RenderingState, VecModel};
use std::cmp::max;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::mpsc;
use strum::IntoEnumIterator;
use wgpu::SurfaceConfiguration;
use wgpu::TextureFormat;
use wgpu::TextureUsages;
use wgpu::{Features, Limits};
use wgpu_canvas::wgpu_canvas::DefaultWgpuCanvas;
use wgpu_canvas::{PreviewType, PREVIEW_TYPE, SHADOWS_ENABLED, SHADOWS_TEX_SIZE, SSAO_ENABLED};

slint::include_modules!();

enum SlintMapEvent {
    VerticalScroll(f32),
    FollowMode(bool),
    FeatureEnabled(Feature, bool),
    BtnAction(Action, i32),
}

fn main() {
    env_logger::init();

    let (slint_map_event_sender, slint_map_event_receiver) = mpsc::channel();

    let mut wgpu_settings = WGPUSettings::default();
    wgpu_settings.device_required_features = Features::CLEAR_TEXTURE | Features::IMMEDIATES;
    wgpu_settings.device_required_limits = Limits::downlevel_defaults();
    wgpu_settings.device_required_limits.max_immediate_size = 4;

    slint::BackendSelector::new()
        .require_wgpu_29(WGPUConfiguration::Automatic(wgpu_settings))
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

    let items: Vec<slint::SharedString> = PreviewType::iter().map(move |preview_type| preview_type
        .to_string()
        .into()).collect();
    ui.set_preview_type_items(Rc::new(VecModel::from(items)).into());

    if max(screen_size.width, screen_size.height) <= 1024 {
        unsafe { SHADOWS_TEX_SIZE = (1024, 1024); }
    }

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

    let mut prev_pinch_scale: Option<Scale> = None;
    let mut prev_pan_state: Option<PanState> = None;
    ui.window()
        .set_rendering_notifier(move |state, graphics_api: &GraphicsAPI| {
            match state {
                RenderingState::RenderingSetup => match graphics_api {
                    GraphicsAPI::WGPU29 { device, queue, .. } => {
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
                            desired_maximum_frame_latency: 2,
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

                            ui_weak.on_preview_type(move |preview_type| {
                                unsafe { PREVIEW_TYPE = PreviewType::from_str(preview_type.as_str()).unwrap(); }
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
                        while let Ok(event) = slint_map_event_receiver.try_recv() {
                            match event {
                                SlintMapEvent::VerticalScroll(delta_y) => {
                                    shashlik_map.pitch_delta(delta_y);
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
                                },
                            };
                        }
                        let curr_pan_state = app.get_current_pan_state();
                        if curr_pan_state.pressed {
                            if let Some(prev_pan) = &prev_pan_state {
                                let delta_x = -(curr_pan_state.x - prev_pan.x) / (14.4 * dpi);
                                let delta_y = -(curr_pan_state.y - prev_pan.y) / (14.4 * dpi);
                                shashlik_map.pan_delta(delta_x, delta_y);
                            }
                            prev_pan_state = Some(curr_pan_state.clone());
                        } else {
                            prev_pan_state = None;
                        }

                        let curr_pinch_scale = app.get_current_scale();
                        if let Some(prev_scale) = &prev_pinch_scale {
                            let delta = curr_pinch_scale.value / prev_scale.value - 1.0;
                            if curr_pinch_scale.value > 0.0 && delta != 0.0 {
                                shashlik_map.zoom_delta(94.0 * delta, (curr_pinch_scale.x, curr_pinch_scale.y));
                            }
                        }

                        if curr_pinch_scale.value > 0.0 {
                            prev_pinch_scale = Some(curr_pinch_scale.clone());
                        } else {
                            prev_pinch_scale = None;
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
