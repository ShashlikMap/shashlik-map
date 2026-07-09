use iced::widget::image::{self, Image};
use iced::widget::{button, center, column, stack, text};
use iced::{window, Element, Length, Subscription, Task};
use map::feature_processor::ShashlikFeatureProcessor;
use map::tiles::default_tiles_provider::DefaultTilesProvider;
use map::ShashlikMap;
use osm::source::reqwest_source::ReqwestSource;
use osm::tiles::TileStore;
use renderer_cpu::CpuRenderer;
use std::time::Instant;

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
    shashlik_map: ShashlikMap<CpuRenderer, DefaultTilesProvider<ShashlikFeatureProcessor>>,
    image_handle: image::Handle,
}

#[derive(Debug, Clone)]
enum Message {
    HardwareTick(Instant),
    Nothing,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let tiles_provider = DefaultTilesProvider::new(
            Box::new(TileStore::new(ReqwestSource::new())),
            ShashlikFeatureProcessor::new(),
            1.0,
        );

        let shashlik_map = pollster::block_on({
            let renderer = CpuRenderer::new();
            ShashlikMap::new(renderer, tiles_provider)
        }).unwrap();

        let initial_handle = image::Handle::from_rgba(400, 400, vec![0; 400 * 400 * 4]);
        (
            Self {
                shashlik_map,
                image_handle: initial_handle,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::HardwareTick(_frame_time) => {
                const WIDTH: u32 = 400;
                const HEIGHT: u32 = 400;
                let pixmap = self.shashlik_map.update_and_render().unwrap();
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
