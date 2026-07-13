use renderer_common::PreviewType;
use slint::{PhysicalSize, VecModel};
use std::rc::Rc;
use strum::IntoEnumIterator;

#[cfg(feature = "linux-cpu")]
mod main_cpu;
#[cfg(feature = "linux-cpu")]
use crate::main_cpu::launch_internal;
#[cfg(feature = "linux-cpu")]
use crate::main_cpu::prepare;

#[cfg(any(feature = "linux-gpu", target_os = "macos"))]
mod main_gpu;
#[cfg(any(feature = "linux-gpu", target_os = "macos"))]
use crate::main_gpu::launch_internal;
#[cfg(any(feature = "linux-gpu", target_os = "macos"))]
use crate::main_gpu::prepare;

slint::include_modules!();

enum SlintMapEvent {
    VerticalScroll(f32),
    FollowMode(bool),
    FeatureEnabled(Feature, bool),
    BtnAction(Action, i32),
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

const ZOOM_SPEED: f32 = 0.02;
const PAN_SPEED: f32 = 10.0;

fn main() {
    env_logger::init();

    #[cfg(target_os = "linux")]
    unsafe {
        std::env::set_var("SLINT_BACKEND", "linuxkms");
        std::env::set_var("SLINT_DEBUG_PERFORMANCE", "refresh_full_speed,overlay");
    }

    prepare();

    let ui = ShashlikUI::new().unwrap();
    let mut screen_size = ui.window().size();
    println!("screen size: {:?}", screen_size);
    if screen_size.width == 0 || screen_size.height == 0 {
        screen_size = PhysicalSize::new(2000, 1200);
    }
    ui.set_screen_width(screen_size.width as i32);
    ui.set_screen_height(screen_size.height as i32);

    let items: Vec<slint::SharedString> = PreviewType::iter()
        .map(move |preview_type| preview_type.to_string().into())
        .collect();
    ui.set_preview_type_items(Rc::new(VecModel::from(items)).into());

    // keep the timer here otherwise it'll be dropped
    let render_timer = slint::Timer::default();
    launch_internal(&ui, &render_timer);

    ui.run().unwrap();
}
