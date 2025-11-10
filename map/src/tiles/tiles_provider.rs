use std::collections::HashSet;
use crate::tiles::tile_data::TileData;
use futures::Stream;
use geo_types::{Coord, Rect};
pub trait TilesProvider {
    fn abc(&mut self, zoom_level: i32);
    fn load(&mut self, area_latlon: Rect, zoom_level: i32);
    fn tiles(&mut self) -> impl Stream<Item = (Option<TileData>, HashSet<String>)> + Send + 'static;
    
    fn lat_lon_to_world(_lat_lon: &Coord<f64>) -> Coord<f64> {
        (0.0, 0.0).into()
    }
    fn world_to_lat_lon(_lat_lon: &Coord<f64>) -> Coord<f64> {
        (0.0, 0.0).into()
    }
}

