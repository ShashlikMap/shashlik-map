use map::tiles::old_tiles_provider::OldTilesProvider;
use std::sync::mpsc;
use native_dialog::DialogBuilder;
use osm::source::reqwest_source::ReqwestSource;
use winit::event_loop::EventLoop;
use winit_run::{App, CustomUIEvent};

slint::include_modules!();

fn main() {
    env_logger::init();

    let (sender, receiver) = mpsc::channel();

    let app = App::new(
        Box::new(|| OldTilesProvider::new(ReqwestSource::new())),
        receiver,
    );
    let event_loop = EventLoop::with_user_event();

    slint::platform::set_platform(Box::new(
        i_slint_backend_winit::Backend::builder()
            .with_event_loop_builder(event_loop)
            .with_custom_application_handler(Box::new(app))
            .build()
            .unwrap(),
    ))
    .unwrap();

    let ui = AppWindow::new().unwrap();
    let sender_clone = sender.clone();
    ui.on_load_button_click(move || {
        sender_clone.send(CustomUIEvent::Load).unwrap();
    });
    let sender_clone = sender.clone();
    ui.on_open_kml_button_click(move || {
        let path = DialogBuilder::file()
            .set_location("~/Desktop")
            .add_filter("KML", ["kml"])
            .open_single_file()
            .show()
            .unwrap();
        if let Some(path) = path {
            sender_clone.send(CustomUIEvent::KMLPath(path)).unwrap();
        }
    });

    ui.run().unwrap();
}
