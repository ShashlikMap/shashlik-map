use iced::widget::{Column, button, column, text};
use iced::window;

pub fn main() -> iced::Result {
    iced::application(u64::default, update, view)
        .window(window::Settings {
            fullscreen: true,
            decorations: false,
            ..Default::default()
        })
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Increment,
}

fn update(value: &mut u64, message: Message) {
    match message {
        Message::Increment => *value += 1,
    }
}

fn view(value: &u64) -> Column<Message> {
    column![text(value), button("+").on_press(Message::Increment),]
}
