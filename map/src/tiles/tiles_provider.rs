use std::collections::HashSet;
use crate::tiles::tile_data::TileData;
use futures::Stream;
use geo_types::{Coord, Polygon, Rect};

pub enum TilesMessage {
    TilesData(Vec<TileData>),
    ToRemove(HashSet<String>),
}

pub trait TilesProvider {
    
    fn load(&mut self, area_latlon: Rect, area_poly: Polygon<f64>, zoom_level: i32);
    
    fn tiles(&mut self) -> impl Stream<Item = TilesMessage> + Send + 'static;
    
    fn lat_lon_to_world(_lat_lon: &Coord<f64>) -> Coord<f64> {
        (0.0, 0.0).into()
    }
    fn world_to_lat_lon(_lat_lon: &Coord<f64>) -> Coord<f64> {
        (0.0, 0.0).into()
    }
}

