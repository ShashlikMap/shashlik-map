use crate::ShashlikUI;
use map::feature_processor::ShashlikFeatureProcessor;
use map::tiles::default_tiles_provider::DefaultTilesProvider;
use map::tiles::mvt::mvt_tile_store::MvtTileStore;
use map::{ShashlikMap, DEFAULT_FONT_DATA};
use renderer_common::fps::FpsCounter;
use renderer_cpu::CpuRenderer;
use skia_safe::{AlphaType, ColorType};
use slint::platform::Key;
use slint::{ComponentHandle, Image, PhysicalSize, SharedPixelBuffer, SharedString};
use std::sync::{Arc, RwLock};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};
use osm::map::{HighwayKind, LineKind, MapGeomObjectKind, MapPointObjectKind, NatureKind};
use map::route::RouteCosting;

enum Interaction {
    Route,
    ZoomIn,
    ZoomOut,
    Left,
    Right,
    Up,
    Down,
    None,
}

const ZOOM_SPEED: f32 = 0.02;
const PAN_SPEED: f32 = 10.0;

const MAX_FPS: f32 = 24.0;

pub fn prepare() {
    unsafe {
        std::env::set_var("SLINT_NO_ACCELERATION", "1");
    }
    slint::BackendSelector::new()
        .renderer_name("skia-software".into())
        .select()
        .expect("Unable to create Slint backend with Software renderer");
}

pub fn launch_internal(ui: &ShashlikUI) {
    let mut screen_size = ui.window().size();
    println!("cpu actual screen size: {:?}", screen_size);
    // fyi, better use a small screen for CPU only renderer
    screen_size = PhysicalSize::new(800, 480);
    ui.set_screen_width(screen_size.width as i32);
    ui.set_screen_height(screen_size.height as i32);

    let texture_width = ui.get_requested_texture_width();
    let texture_height = ui.get_requested_texture_height();
    println!(
        "texture width: {} and height: {}",
        texture_width, texture_height
    );

    let width = texture_width as u32;
    let height = texture_height as u32;

    let tiles_provider = DefaultTilesProvider::new(
        Box::new(MvtTileStore::new()),
        ShashlikFeatureProcessor::new(false, |zoom_level, kind| {
            match kind {
                MapGeomObjectKind::Nature(kind) => match kind {
                    NatureKind::Park => zoom_level >= 12,
                    _ => true,
                }
                MapGeomObjectKind::Building(_) => zoom_level >= 15,
                MapGeomObjectKind::Way(info) => {
                    match info.line_kind {
                        LineKind::Highway { kind } => {
                            match kind {
                                HighwayKind::Motorway => zoom_level >= 5,
                                HighwayKind::Trunk => zoom_level >= 10,
                                HighwayKind::Primary => zoom_level >= 11,
                                HighwayKind::Secondary => zoom_level >= 12,
                                HighwayKind::Service => zoom_level >= 15,
                                _ => zoom_level >= 14
                            }
                        },
                        LineKind::Railway { .. } => zoom_level >= 14,
                        // no line label on CPU only devices
                        LineKind::Label => false
                    }
                }
                MapGeomObjectKind::Poi(info) => {
                    match info.kind {
                        MapPointObjectKind::PopArea(_) => true,
                        MapPointObjectKind::TrainStation(_) => true,
                        _ => false
                    }
                },
                _ => true
            }
        }),
        1.0,
    );

    let mut shashlik_map = pollster::block_on({
        let renderer = CpuRenderer::new(width, height, &DEFAULT_FONT_DATA);
        ShashlikMap::new(renderer, tiles_provider)
    })
    .unwrap();
    shashlik_map.set_camera_follow_mode(false);
    shashlik_map.set_current_pitch(90.0);
    shashlik_map.resize(width, height);

    let ui_weak = ui.as_weak();

    let interaction = Arc::new(RwLock::new(Interaction::None));
    let press_interaction = Arc::clone(&interaction);
    ui.on_key_pressed(move |key_event| {
        let mut interaction = press_interaction.write().unwrap();
        if key_event.text == SharedString::from(Key::LeftArrow) {
            *interaction = Interaction::Left;
        } else if key_event.text == SharedString::from(Key::RightArrow) {
            *interaction = Interaction::Right;
        } else if key_event.text == SharedString::from(Key::UpArrow) {
            *interaction = Interaction::Up;
        } else if key_event.text == SharedString::from(Key::DownArrow) {
            *interaction = Interaction::Down;
        } else {
            let key_str = key_event.text.as_str();
            if let Some(key_char) = key_str.chars().next() {
                match key_char {
                    'a' | 'A' => {
                        *interaction = Interaction::ZoomIn;
                    }
                    'z' | 'Z' => {
                        *interaction = Interaction::ZoomOut;
                    }
                    'r' | 'R' => {
                        *interaction = Interaction::Route;
                    }
                    _ => {}
                }
            }
        }
    });

    let release_interaction = Arc::clone(&interaction);
    ui.on_key_released(move |_| {
        let mut interaction = release_interaction.write().unwrap();
        *interaction = Interaction::None;
    });

    spawn(move || {
        let mut fps_counter: FpsCounter<100> = FpsCounter::new();
        loop {
            let frame_start = Instant::now();
            if let Ok(interaction) = interaction.try_read() {
                match *interaction {
                    Interaction::Route => {
                        shashlik_map
                            .create_route_to_from_screen_center(RouteCosting::Auto);
                    }
                    Interaction::ZoomIn => {
                        shashlik_map.zoom_delta(
                            1.0 + ZOOM_SPEED,
                            ((width as f32) * 0.5, (height as f32) * 0.5),
                        );
                    }
                    Interaction::ZoomOut => {
                        shashlik_map.zoom_delta(
                            1.0 - ZOOM_SPEED,
                            ((width as f32) * 0.5, (height as f32) * 0.5),
                        );
                    }
                    Interaction::Left => {
                        shashlik_map.pan_delta(-PAN_SPEED, 0.0);
                    }
                    Interaction::Right => {
                        shashlik_map.pan_delta(PAN_SPEED, 0.0);
                    }
                    Interaction::Up => {
                        shashlik_map.pan_delta(0.0, -PAN_SPEED);
                    }
                    Interaction::Down => {
                        shashlik_map.pan_delta(0.0, PAN_SPEED);
                    }
                    Interaction::None => {}
                }
            }

            let mut pixel_buffer =
                SharedPixelBuffer::<slint::Rgba8Pixel>::new(width as u32, height as u32);
            let raw_bytes = pixel_buffer.make_mut_bytes();
            let row_bytes = (width * 4) as usize;
            let image_info = skia_safe::ImageInfo::new(
                skia_safe::ISize::new(width as i32, height as i32),
                ColorType::RGBA8888,
                AlphaType::Premul,
                None,
            );

            if let Some(mut surface) =
                skia_safe::surfaces::wrap_pixels(&image_info, raw_bytes, Some(row_bytes), None)
            {
                let canvas = surface.canvas();
                shashlik_map.update_and_render(canvas);
            }

            let ui_weak = ui_weak.clone();
            let fps = fps_counter.update();
            slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    let image = Image::from_rgba8_premultiplied(pixel_buffer.clone());
                    ui.set_texture(image);
                    ui.set_fps_text(format!("FPS: {:.1}", fps).into());
                    ui.window().request_redraw();
                }
            })
            .expect("Can't execute invoke_from_event_loop");

            let work_duration = frame_start.elapsed();
            if let Some(remaining_sleep_time) = Duration::from_millis((1000.0 / MAX_FPS) as u64).checked_sub(work_duration)
            {
                sleep(remaining_sleep_time);
            }
        }
    });
}
