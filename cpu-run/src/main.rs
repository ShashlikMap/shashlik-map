use std::time::Instant;
use slint::{Image, SharedPixelBuffer};

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
            color: white;
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

        // 1. Calculate FPS every second
        frame_count += 1;
        let elapsed_seconds = fps_timer.elapsed().as_secs_f32();
        if elapsed_seconds >= 1.0 {
            let fps = frame_count as f32 / elapsed_seconds;
            window.set_fps_text(format!("FPS: {:.1}", fps).into());

            frame_count = 0;
            fps_timer = Instant::now();
        }

        // 2. Allocate frame memory to build our visual matrix
        let mut pixel_buffer = SharedPixelBuffer::<slint::Rgb8Pixel>::new(width, height);
        let pixels = pixel_buffer.make_mut_bytes();

        // Advance animation index
        animation_tick = animation_tick.wrapping_add(1);

        // 3. Draw a changing color landscape via the CPU
        for y in 0..height {
            for x in 0..width {
                let pixel_index = ((y * width + x) * 3) as usize;

                // Mixing the physical position and animation clock
                let r = ((x * 255 / width) as u32).wrapping_add(animation_tick) as u8;
                let g = ((y * 255 / height) as u32).wrapping_add(animation_tick * 2) as u8;
                let b = 128_u8;

                pixels[pixel_index] = r;
                pixels[pixel_index + 1] = g;
                pixels[pixel_index + 2] = b;
            }
        }

        // 4. Update the display property
        let image = Image::from_rgb8(pixel_buffer);
        window.set_render_texture(image);

        // 5. Explicitly request a redraw from Slint
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
