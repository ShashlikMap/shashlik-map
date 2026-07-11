use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::image::{self, Image};
use iced::widget::{center, container, stack, text};
use iced::{Color, Element, Event, Length, Size, Subscription, Task, event, keyboard, window};
use map::ShashlikMap;
use map::feature_processor::ShashlikFeatureProcessor;
use map::tiles::default_tiles_provider::DefaultTilesProvider;
use map::tiles::mvt::mvt_tile_store::MvtTileStore;
use renderer_cpu::{CpuRenderer, FpsCounter};
use std::time::Instant;

fn main() -> iced::Result {
    env_logger::init();
    iced::application(App::new, App::update, App::view)
        .window(window::Settings {
            size: Size::new(CpuRenderer::WIDTH as f32, CpuRenderer::HEIGHT as f32),
            fullscreen: false,
            decorations: false,
            exit_on_close_request: true,
            ..Default::default()
        })
        .subscription(App::subscription)
        .run()
}

struct App {
    shashlik_map: ShashlikMap<CpuRenderer, DefaultTilesProvider<ShashlikFeatureProcessor>>,
    image_handle: image::Handle,
    interaction: Interaction,
    fps_counter: FpsCounter<100>,
    last_fps: u16,
}

#[derive(Debug, Clone)]
enum Message {
    HardwareTick(Instant),
    KeyboardInput(keyboard::Event),
}

enum Interaction {
    ZoomIn,
    ZoomOut,
    Left,
    Right,
    Up,
    Down,
    None,
}

impl App {
    const ZOOM_SPEED: f32 = 0.02;
    const PAN_SPEED: f32 = 10.0;
    fn new() -> (Self, Task<Message>) {
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

        let initial_handle = image::Handle::from_rgba(
            CpuRenderer::WIDTH,
            CpuRenderer::HEIGHT,
            vec![0; (CpuRenderer::WIDTH * CpuRenderer::HEIGHT * 4) as usize],
        );
        (
            Self {
                shashlik_map,
                image_handle: initial_handle,
                interaction: Interaction::None,
                fps_counter: FpsCounter::new(),
                last_fps: 0u16,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::HardwareTick(_frame_time) => {
                match &self.interaction {
                    Interaction::ZoomIn => {
                        self.shashlik_map.zoom_delta(
                            1.0 + Self::ZOOM_SPEED,
                            (
                                (CpuRenderer::WIDTH as f32) * 0.5,
                                (CpuRenderer::HEIGHT as f32) * 0.5,
                            ),
                        );
                    }
                    Interaction::ZoomOut => {
                        self.shashlik_map.zoom_delta(
                            1.0 - Self::ZOOM_SPEED,
                            (
                                (CpuRenderer::WIDTH as f32) * 0.5,
                                (CpuRenderer::HEIGHT as f32) * 0.5,
                            ),
                        );
                    }
                    Interaction::Left => {
                        self.shashlik_map.pan_delta(-Self::PAN_SPEED, 0.0);
                    }
                    Interaction::Right => {
                        self.shashlik_map.pan_delta(Self::PAN_SPEED, 0.0);
                    }
                    Interaction::Up => {
                        self.shashlik_map.pan_delta(0.0, -Self::PAN_SPEED);
                    }
                    Interaction::Down => {
                        self.shashlik_map.pan_delta(0.0, Self::PAN_SPEED);
                    }
                    Interaction::None => {}
                }

                let pixmap = self.shashlik_map.update_and_render().unwrap();
                let width = pixmap.width();
                let height = pixmap.height();
                let raw_rgba_pixels = pixmap.take();
                self.image_handle = image::Handle::from_rgba(width, height, raw_rgba_pixels);
                self.last_fps = self.fps_counter.update() as u16;
            }
            Message::KeyboardInput(event) => match event {
                keyboard::Event::KeyPressed { key, .. } => match key {
                    Key::Named(Named::ArrowLeft) => {
                        self.interaction = Interaction::Left;
                    }
                    Key::Named(Named::ArrowRight) => {
                        self.interaction = Interaction::Right;
                    }
                    Key::Named(Named::ArrowUp) => {
                        self.interaction = Interaction::Up;
                    }
                    Key::Named(Named::ArrowDown) => {
                        self.interaction = Interaction::Down;
                    }
                    Key::Character(c) if c == "z" || c == "Z" => {
                        self.interaction = Interaction::ZoomIn;
                    }
                    Key::Character(c) if c == "x" || c == "X" => {
                        self.interaction = Interaction::ZoomOut;
                    }
                    _ => {}
                },
                keyboard::Event::KeyReleased { .. } => {
                    self.interaction = Interaction::None;
                }
                _ => {}
            },
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            iced::window::frames().map(Message::HardwareTick),
            event::listen_with(|event, _status, _window_id| match event {
                Event::Keyboard(kbd_event) => Some(Message::KeyboardInput(kbd_event)),
                _ => None,
            }),
        ])
    }

    fn view(&self) -> Element<'_, Message> {
        stack![
            center(
                Image::new(self.image_handle.clone())
                    .width(Length::Shrink)
                    .height(Length::Shrink)
            ),
            container(
                text(format!("FPS {}", self.last_fps))
                    .size(20)
                    .color(Color::BLACK)
            )
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Top)
            .padding(10)
        ]
        .into()
    }
}
