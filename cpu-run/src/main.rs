use iced::widget::image::{self, Image};
use iced::widget::{Column, Container, button, center, column, stack, text};
use iced::{Element, Length, Subscription, Task, window};
use std::time::Instant;
use tiny_skia::{Paint, Pixmap, Rect, Transform};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window(window::Settings {
            fullscreen: true,
            decorations: false,
            ..Default::default()
        })
        .subscription(App::subscription)
        .run()
}

struct App {
    image_handle: image::Handle,
    start_time: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    HardwareTick(Instant),
    Nothing,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let initial_handle = image::Handle::from_rgba(400, 400, vec![0; 400 * 400 * 4]);
        (
            Self {
                image_handle: initial_handle,
                start_time: Instant::now(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::HardwareTick(_frame_time) => {
                const WIDTH: u32 = 400;
                const HEIGHT: u32 = 400;
                let mut pixmap = Pixmap::new(WIDTH, HEIGHT).unwrap();
                pixmap.fill(tiny_skia::Color::from_rgba8(30, 30, 30, 255));

                let time_elapsed = self.start_time.elapsed().as_secs_f32();
                let x_offset = (time_elapsed.sin() * 100.0) + 150.0;

                let mut paint = Paint::default();
                paint.set_color(tiny_skia::Color::from_rgba8(46, 204, 113, 255)); // Green
                paint.anti_alias = true;

                if let Some(rect) = Rect::from_xywh(x_offset, 150.0, 100.0, 100.0) {
                    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                }

                let raw_rgba_pixels = pixmap.take();
                self.image_handle = image::Handle::from_rgba(WIDTH, HEIGHT, raw_rgba_pixels);
            }
            Message::Nothing => {}
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::window::frames().map(Message::HardwareTick)
    }

    fn view(&self) -> Element<'_, Message> {
        stack![
            center(
                Image::new(self.image_handle.clone())
                    .width(Length::Shrink)
                    .height(Length::Shrink)
            ),
            center(column![
                text("Test"),
                button("+").on_press(Message::Nothing),
            ])
        ]
        .into()
    }
}
