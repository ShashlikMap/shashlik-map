use map::feature_processor::ShashlikFeatureProcessor;
use map::tiles::default_tiles_provider::DefaultTilesProvider;
use map::tiles::mvt::mvt_tile_store::MvtTileStore;
use map::ShashlikMap;
use renderer_cpu::{CpuRenderer, FpsCounter};
use skia_safe::{AlphaType, ColorType};
use slint::{Image, SharedPixelBuffer};

slint::slint! {
    export component MainWindow inherits Window {
        in-out property <image> render_texture;
        in-out property <string> fps_text: "FPS: --";

        width: 1024px;
        height: 600px;
        background: black;

        Image {
            source: root.render_texture;
            width: 100%;
            height: 100%;
        }

        Text {
            text: root.fps_text;
            color: red;
            font-size: 24px;
            font-weight: 700;
            x: 20px;
            y: 20px;
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    env_logger::init();

    unsafe {
        std::env::set_var("SLINT_BACKEND", "linuxkms");
    }
    unsafe {
        std::env::set_var("SLINT_NO_ACCELERATION", "1");
    }

    let main_window = MainWindow::new()?;
    let main_window_weak = main_window.as_weak();

    let width = 1024;
    let height = 600;

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

    let mut fps_counter: FpsCounter<100> = FpsCounter::new();

    let render_timer = slint::Timer::default();
    render_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(0),
        move || {
            let Some(window) = main_window_weak.upgrade() else {
                return;
            };

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

            if let Some(mut surface) = skia_safe::surfaces::wrap_pixels(&image_info, raw_bytes, Some(row_bytes), None) {
                let canvas = surface.canvas(); // This acquires the native drawing context
                shashlik_map.update_and_render(canvas);

                // shashlik_map.pan_delta(0.3, 0.0);

                // shashlik_map.zoom_delta(
                //     1.0 - 0.002,
                //     (
                //         (CpuRenderer::WIDTH as f32) * 0.5,
                //         (CpuRenderer::HEIGHT as f32) * 0.5,
                //     ),
                // );
            }

            let image = Image::from_rgba8(pixel_buffer);
            window.set_render_texture(image);
            window.window().request_redraw();
        },
    );

    main_window.run()
}
