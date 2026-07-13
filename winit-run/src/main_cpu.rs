use map::ShashlikMap;
use map::feature_processor::ShashlikFeatureProcessor;
use map::tiles::default_tiles_provider::DefaultTilesProvider;
use map::tiles::mvt::mvt_tile_store::MvtTileStore;
use renderer_cpu::{CpuRenderer, FpsCounter};
use skia_safe::{AlphaType, ColorType};
use slint::platform::Key;
use slint::{ComponentHandle, Image, SharedPixelBuffer, SharedString, Timer};
use std::sync::{Arc, RwLock};
use crate::ShashlikUI;
use crate::Interaction;
use crate::PAN_SPEED;
use crate::ZOOM_SPEED;

pub fn prepare() {
    unsafe {
        std::env::set_var("SLINT_NO_ACCELERATION", "1");
    }
    slint::BackendSelector::new()
        .renderer_name("skia-software".into())
        .select().expect("Unable to create Slint backend with Software renderer");
}

pub fn launch_internal(ui: &ShashlikUI, render_timer: &Timer) {
    let width = 780;
    let height = 568;

    let tiles_provider = DefaultTilesProvider::new(
        Box::new(MvtTileStore::new()),
        ShashlikFeatureProcessor::new(false),
        1.0,
    );

    let mut shashlik_map = pollster::block_on({
        let renderer = CpuRenderer::new();
        ShashlikMap::new(renderer, tiles_provider)
    })
        .unwrap();
    shashlik_map.set_camera_follow_mode(false);
    shashlik_map.set_current_pitch(90.0);
    shashlik_map.resize(CpuRenderer::WIDTH, CpuRenderer::HEIGHT);

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

    let mut fps_counter: FpsCounter<100> = FpsCounter::new();
    render_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(0),
        move || {
            let Some(window) = ui_weak.upgrade() else {
                return;
            };
            if let Ok(interaction) = interaction.try_read() {
                match *interaction {
                    Interaction::ZoomIn => {
                        shashlik_map.zoom_delta(
                            1.0 + ZOOM_SPEED,
                            (
                                (CpuRenderer::WIDTH as f32) * 0.5,
                                (CpuRenderer::HEIGHT as f32) * 0.5,
                            ),
                        );
                    }
                    Interaction::ZoomOut => {
                        shashlik_map.zoom_delta(
                            1.0 - ZOOM_SPEED,
                            (
                                (CpuRenderer::WIDTH as f32) * 0.5,
                                (CpuRenderer::HEIGHT as f32) * 0.5,
                            ),
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

            window.set_fps_text(format!("FPS: {:.1}", fps_counter.update()).into());

            let mut pixel_buffer = SharedPixelBuffer::<slint::Rgba8Pixel>::new(width, height);

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

            // TODO Figure out if it should be from_rgba8_premultiplied
            let image = Image::from_rgba8(pixel_buffer);
            window.set_texture(image);
            window.window().request_redraw();
        },
    );
}