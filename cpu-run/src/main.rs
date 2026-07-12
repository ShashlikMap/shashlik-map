use std::time::Instant;
use slint::{Image, SharedPixelBuffer};
use tiny_skia::{Color, LineCap, LineJoin, Paint, PathBuilder, PixmapMut, Stroke};

slint::slint! {
    export component MainWindow inherits Window { // Uses 'inherits' instead of 'extends'
        in-out property <image> render_texture;
        // Text property to show the frame calculation
        in-out property <string> fps_text: "FPS: --";

        width: 1024px;
        height: 600px;
        background: black;

        // Draw the custom CPU pixel buffer
        Image {
            source: root.render_texture;
            width: 100%;
            height: 100%;
        }

        // Draw the text overlay on top of the image
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

    // Force Slint to use CPU software rendering and LinuxKMS
    unsafe { std::env::set_var("SLINT_BACKEND", "linuxkms"); }
    unsafe { std::env::set_var("SLINT_NO_ACCELERATION", "1"); }

    let main_window = MainWindow::new()?;
    let main_window_weak = main_window.as_weak();

    // Constant frame measurements
    let width = 1024;
    let height = 600;

    // Track performance metrics across ticks
    let mut last_frame_time = Instant::now();
    let mut frame_count = 0;
    let mut fps_timer = Instant::now();

    // Changing value to feed into our math to make things move
    let mut animation_tick: u32 = 0;

    // Set up a repeating timer that fires as fast as possible (0ms interval)
    let render_timer = slint::Timer::default();
    render_timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(0), move || {
        let Some(window) = main_window_weak.upgrade() else { return; };

        // 1. Calculate Frame Rates
        frame_count += 1;
        let elapsed_seconds = fps_timer.elapsed().as_secs_f32();
        if elapsed_seconds >= 1.0 {
            let fps = frame_count as f32 / elapsed_seconds;
            window.set_fps_text(format!("FPS: {:.1}", fps).into());
            frame_count = 0;
            fps_timer = Instant::now();
        }

        // 2. Setup a 4-channel RGBA pixel buffer for tiny-skia compliance
        let mut pixel_buffer = SharedPixelBuffer::<slint::Rgba8Pixel>::new(width, height);
        animation_tick = animation_tick.wrapping_add(1);

        // 3. Wrap Slint's raw memory slice into a tiny-skia canvas surface
        let raw_bytes = pixel_buffer.make_mut_bytes();

        // 4. Fill a changing color gradient background using raw array loops
        // for y in 0..height {
        //     for x in 0..width {
        //         let pixel_index = ((y * width + x) * 4) as usize; // 4 bytes per pixel now
        //
        //         let r = ((x * 255 / width) as u32).wrapping_add(animation_tick) as u8;
        //         let g = ((y * 255 / height) as u32).wrapping_add(animation_tick * 2) as u8;
        //
        //         raw_bytes[pixel_index] = r;     // R
        //         raw_bytes[pixel_index + 1] = g; // G
        //         raw_bytes[pixel_index + 2] = 128;// B
        //         raw_bytes[pixel_index + 3] = 255;// Alpha (Fully Opaque)
        //     }
        // }

        let mut pixmap = PixmapMut::from_bytes(raw_bytes, width, height).unwrap();
        pixmap.fill(Color::WHITE);
        // 5. Build a dynamic vector Path using tiny-skia's PathBuilder API
        let mut pb = PathBuilder::new();
        let points_count = 50;
        let t_offset = animation_tick as f32 * 0.06;

        // for i in 0..100 {
        //     pb.move_to(0.0, (i * 5) as f32);
        //     pb.line_to(600.0, (i * 5) as f32);
        // }

        for i in 0..200 {
            pb.move_to((i * 5) as f32, 0.0);
            pb.line_to((i * 5) as f32, 600.0);
        }


        // for i in 0..points_count {
        //     let ratio = i as f32 / (points_count - 1) as f32;
        //     let x = ratio * 800.0;
        //
        //     // Generate a moving wave vector sequence
        //     let wave_y = (ratio * 6.28 * 2.0 + t_offset).sin() * 120.0;
        //     let y = 300.0 + wave_y;
        //
        //     if i == 0 {
        //         pb.move_to(x, y);
        //     } else {
        //         pb.line_to(x, y);
        //     }
        // }

        // Finalize the mathematical path object
        let path = pb.finish().unwrap();

        // 6. Define the path coloring property
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba8(0, 240, 255, 255)); // Bright neon cyan
        paint.anti_alias = true; // Makes the line crisp and edge-smoothed

        // 7. Define the Stroke properties (Weight, Joints, Caps)
        let mut stroke = Stroke::default();
        stroke.width = 1.0;
        stroke.line_cap = LineCap::Round;  // Beautiful rounded line ends
        stroke.line_join = LineJoin::Round; // Smooth joints at sharp turns

        // 8. Command tiny-skia to rasterize the path vector into the buffer
        pixmap.stroke_path(&path, &paint, &stroke, tiny_skia::Transform::identity(), None);

        // 9. Hand the final frame over to the Slint UI interface
        let image = Image::from_rgba8(pixel_buffer);
        window.set_render_texture(image);
        window.window().request_redraw();
    });

    main_window.run()

    // iced::application(App::new, App::update, App::view)
    //     .window(window::Settings {
    //         size: Size::new(CpuRenderer::WIDTH as f32, CpuRenderer::HEIGHT as f32),
    //         fullscreen: false,
    //         decorations: false,
    //         exit_on_close_request: true,
    //         ..Default::default()
    //     })
    //     .subscription(App::subscription)
    //     .run()
}

// struct App {
//     shashlik_map: ShashlikMap<CpuRenderer, DefaultTilesProvider<ShashlikFeatureProcessor>>,
//     image_handle: image::Handle,
//     interaction: Interaction,
//     fps_counter: FpsCounter<100>,
//     last_fps: u16,
// }
//
// #[derive(Debug, Clone)]
// enum Message {
//     HardwareTick(Instant),
//     KeyboardInput(keyboard::Event),
// }
//
// enum Interaction {
//     ZoomIn,
//     ZoomOut,
//     Left,
//     Right,
//     Up,
//     Down,
//     None,
// }
//
// impl App {
//     const ZOOM_SPEED: f32 = 0.02;
//     const PAN_SPEED: f32 = 10.0;
//     fn new() -> (Self, Task<Message>) {
//         let tiles_provider = DefaultTilesProvider::new(
//             Box::new(MvtTileStore::new()),
//             ShashlikFeatureProcessor::new(false),
//             1.0,
//         );
//
//         let mut shashlik_map = pollster::block_on({
//             let renderer = CpuRenderer::new();
//             ShashlikMap::new(renderer, tiles_provider)
//         })
//         .unwrap();
//         shashlik_map.set_camera_follow_mode(false);
//         shashlik_map.set_current_pitch(90.0);
//         shashlik_map.resize(CpuRenderer::WIDTH, CpuRenderer::HEIGHT);
//
//         let initial_handle = image::Handle::from_rgba(
//             CpuRenderer::WIDTH,
//             CpuRenderer::HEIGHT,
//             vec![0; (CpuRenderer::WIDTH * CpuRenderer::HEIGHT * 4) as usize],
//         );
//         (
//             Self {
//                 shashlik_map,
//                 image_handle: initial_handle,
//                 interaction: Interaction::None,
//                 fps_counter: FpsCounter::new(),
//                 last_fps: 0u16,
//             },
//             Task::none(),
//         )
//     }
//
//     fn update(&mut self, message: Message) -> Task<Message> {
//         match message {
//             Message::HardwareTick(_frame_time) => {
//                 match &self.interaction {
//                     Interaction::ZoomIn => {
//                         self.shashlik_map.zoom_delta(
//                             1.0 + Self::ZOOM_SPEED,
//                             (
//                                 (CpuRenderer::WIDTH as f32) * 0.5,
//                                 (CpuRenderer::HEIGHT as f32) * 0.5,
//                             ),
//                         );
//                     }
//                     Interaction::ZoomOut => {
//                         self.shashlik_map.zoom_delta(
//                             1.0 - Self::ZOOM_SPEED,
//                             (
//                                 (CpuRenderer::WIDTH as f32) * 0.5,
//                                 (CpuRenderer::HEIGHT as f32) * 0.5,
//                             ),
//                         );
//                     }
//                     Interaction::Left => {
//                         self.shashlik_map.pan_delta(-Self::PAN_SPEED, 0.0);
//                     }
//                     Interaction::Right => {
//                         self.shashlik_map.pan_delta(Self::PAN_SPEED, 0.0);
//                     }
//                     Interaction::Up => {
//                         self.shashlik_map.pan_delta(0.0, -Self::PAN_SPEED);
//                     }
//                     Interaction::Down => {
//                         self.shashlik_map.pan_delta(0.0, Self::PAN_SPEED);
//                     }
//                     Interaction::None => {}
//                 }
//
//                 let pixmap = self.shashlik_map.update_and_render().unwrap();
//                 let width = pixmap.width();
//                 let height = pixmap.height();
//                 let raw_rgba_pixels = pixmap.take();
//                 self.image_handle = image::Handle::from_rgba(width, height, raw_rgba_pixels);
//                 self.last_fps = self.fps_counter.update() as u16;
//             }
//             Message::KeyboardInput(event) => match event {
//                 keyboard::Event::KeyPressed { key, .. } => match key {
//                     Key::Named(Named::ArrowLeft) => {
//                         self.interaction = Interaction::Left;
//                     }
//                     Key::Named(Named::ArrowRight) => {
//                         self.interaction = Interaction::Right;
//                     }
//                     Key::Named(Named::ArrowUp) => {
//                         self.interaction = Interaction::Up;
//                     }
//                     Key::Named(Named::ArrowDown) => {
//                         self.interaction = Interaction::Down;
//                     }
//                     Key::Character(c) if c == "z" || c == "Z" => {
//                         self.interaction = Interaction::ZoomIn;
//                     }
//                     Key::Character(c) if c == "x" || c == "X" => {
//                         self.interaction = Interaction::ZoomOut;
//                     }
//                     _ => {}
//                 },
//                 keyboard::Event::KeyReleased { .. } => {
//                     self.interaction = Interaction::None;
//                 }
//                 _ => {}
//             },
//         }
//         Task::none()
//     }
//
//     fn subscription(&self) -> Subscription<Message> {
//         Subscription::batch(vec![
//             iced::window::frames().map(Message::HardwareTick),
//             event::listen_with(|event, _status, _window_id| match event {
//                 Event::Keyboard(kbd_event) => Some(Message::KeyboardInput(kbd_event)),
//                 _ => None,
//             }),
//         ])
//     }
//
//     fn view(&self) -> Element<'_, Message> {
//         stack![
//             center(
//                 Image::new(self.image_handle.clone())
//                     .width(Length::Shrink)
//                     .height(Length::Shrink)
//             ),
//             container(
//                 text(format!("FPS {}", self.last_fps))
//                     .size(20)
//                     .color(Color::BLACK)
//             )
//             .align_x(iced::alignment::Horizontal::Left)
//             .align_y(iced::alignment::Vertical::Top)
//             .padding(10)
//         ]
//         .into()
//     }
// }
