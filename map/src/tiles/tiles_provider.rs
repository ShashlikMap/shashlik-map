use std::collections::HashSet;
use crate::tiles::tile_data::TileData;
use futures::Stream;
use geo_types::{Coord, Polygon, Rect};

pub enum TilesMessage {
    TilesData(Vec<TileData>),
    ToRemove(HashSet<String>),
}

pub trait TilesProvider {
    
    fn load(&mut self, area_lonlat: Rect, area_poly: Polygon<f64>, zoom_level: i32);
    
    fn tiles(&mut self) -> impl Stream<Item = TilesMessage> + Send + 'static;
    
    fn lon_lat_to_world(_lon_lat: &Coord<f64>) -> Coord<f64> {
        (0.0, 0.0).into()
    }
    fn lon_lat_to_world2(_lon_lat: &Coord<f64>, zl: i32) -> Coord<f64> {
        (0.0, 0.0).into()
    }
    fn world_to_lon_lat(_xy: &Coord<f64>) -> Coord<f64> {
        (0.0, 0.0).into()
    }

    fn world_to_lon_lat2(_xy: &Coord<f64>, zl: i32) -> Coord<f64> {
        (0.0, 0.0).into()
    }
}

