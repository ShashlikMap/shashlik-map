use map::tiles::old_tiles_provider::OldTilesProvider;
use old_tiles_gen::source::reqwest_source::ReqwestSource;
use old_tiles_gen::source::tiles_sqlite_store::TilesSQLiteStore;
use std::sync::mpsc;
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
    ui.on_load_button_click(move || {
        sender.send(CustomUIEvent::Load).unwrap();
    });

    ui.run().unwrap();
}
