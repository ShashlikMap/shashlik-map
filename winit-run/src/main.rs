use map::feature_processor::ShashlikFeatureProcessor;
use map::tiles::shashlik_tiles_provider_v0::ShashlikTilesProviderV0;
use map::ShashlikMap;
use osm::source::reqwest_source::ReqwestSource;
use slint::{Image, PhysicalSize, Rgba8Pixel, SharedPixelBuffer, VecModel};
use std::cmp::max;
use std::rc::Rc;
use std::str::FromStr;
use std::time::{Duration, Instant};
use strum::IntoEnumIterator;
use tiny_skia::PixmapMut;
use wgpu_canvas::{PreviewType, SHADOWS_TEX_SIZE};

slint::include_modules!();

enum SlintMapEvent {
    VerticalScroll(f32),
    FollowMode(bool),
    FeatureEnabled(Feature, bool),
    BtnAction(Action, i32),
}

fn main() {
    env_logger::init();

    // let (slint_map_event_sender, slint_map_event_receiver) = mpsc::channel();

    // let mut wgpu_settings = WGPUSettings::default();
    // wgpu_settings.device_required_features = Features::CLEAR_TEXTURE | Features::IMMEDIATES;
    // wgpu_settings.device_required_limits = Limits::downlevel_defaults();
    // wgpu_settings.device_required_limits.max_immediate_size = 4;

    slint::BackendSelector::new()
        // .require_wgpu_29(WGPUConfiguration::Automatic(wgpu_settings))
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
    // let mut shashlik_map = None;

    let mut prev_pinch_scale: Option<Scale> = None;
    let mut prev_pan_state: Option<PanState> = None;

    let tiles_provider = ShashlikTilesProviderV0::new(
        ReqwestSource::new(),
        ShashlikFeatureProcessor::new(),
        dpi,
    );

    let value = ui_weak.clone();
    std::thread::spawn(move || {
        let mut map =
            pollster::block_on(ShashlikMap::new(tiles_provider))
                .unwrap();
        map.resize(texture_width as u32, texture_height as u32);

        // Target framerate tracking variables (~60 FPS)
        let target_fps = 60;
        let frame_duration = Duration::from_secs_f64(1.0 / target_fps as f64);
        let mut frame_count = 0;
        loop {
            let start_time = Instant::now();

            // 2. Process frame generation using zero-copy allocation
            let mut pixel_buffer = SharedPixelBuffer::<Rgba8Pixel>::new(500, 500);
            let raw_bytes = pixel_buffer.make_mut_bytes();

            if let Some(mut pixmap) = PixmapMut::from_bytes(raw_bytes, 500, 500) {
                // Execute draw calls inside the thread
                map.update_and_render_software(&mut pixmap);
                // draw_dynamic_scene(&mut pixmap, frame_count,500, 500);
            }

            // Convert to a Slint Image handle

            // 3. Dispatch the calculated buffer to the UI Event Loop
            let app_clone = value.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = app_clone.upgrade() {
                    let frame_image = Image::from_rgba8_premultiplied(pixel_buffer);
                    app.set_texture(frame_image);
                }
            });

            frame_count += 1;
            let elapsed = start_time.elapsed();
            if elapsed < frame_duration {
                std::thread::sleep(frame_duration - elapsed);
            }
        }
    });

    ui.run().unwrap();
}
