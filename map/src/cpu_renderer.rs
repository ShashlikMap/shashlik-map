use crate::tiles::tiles_provider::TilesProvider;
use geo_types::Coord;
use std::time::Instant;
use tiny_skia::{Paint, Pixmap, Rect, Transform};

pub trait NewRenderer<T> {
    fn new_update_and_render(&mut self) -> T;
}

pub struct NewTempCpuRenderer<T: TilesProvider> {
    pub(crate) tiles_provider: T,
    start_time: Instant,
}
impl<T: TilesProvider> NewTempCpuRenderer<T> {
    pub fn new(tiles_provider: T) -> Self {
        Self {
            tiles_provider,
            start_time: Instant::now(),
        }
    }
}
impl<T: TilesProvider> NewRenderer<Pixmap> for NewTempCpuRenderer<T> {
    fn new_update_and_render(&mut self) -> Pixmap {
        let initial_coord: Coord<f64> = (139.757080078125, 35.68798828125).into();
        let world_coord = self
            .tiles_provider
            .inner_converter()
            .lon_lat_to_world(&initial_coord, 7);
        println!("NEW RENDERER UPDATE: {:?}", world_coord);

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

        pixmap
    }
}
