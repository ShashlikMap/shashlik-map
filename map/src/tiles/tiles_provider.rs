use crate::tiles::tile_data::TileData;
use futures::Stream;
use geo_types::{Coord, Polygon, Rect};
use glam::DVec3;
use osm::map::{MapGeomObject, MapGeometry};
use osm::tiles::TileKey;
use std::collections::HashSet;

pub enum TilesMessage {
    TilesData(Vec<TileData>),
    ToRemove(HashSet<String>),
}

pub trait MercatorConverter {
    fn lon_lat_to_world(lon_lat: &Coord<f64>, zoom_level: i32) -> Coord<f64>;
    fn world_to_lon_lat(xy: &Coord<f64>, zoom_level: i32) -> Coord<f64>;
}

pub trait TilesProviderStore<C: MercatorConverter>: Send + Sync {
    fn tile_position_bbox(&self, tile_key: &TileKey, bbox_scale: f64) -> (DVec3, Rect);
    fn load(&self, tile_key: &TileKey) -> Vec<(MapGeomObject, MapGeometry<f32>)>;
}

pub trait TilesProvider: MercatorConverter {
    
    fn load(&mut self, area_lonlat: Rect, area_poly: Polygon<f64>, zoom_level: i32);
    
    fn tiles(&mut self) -> impl Stream<Item = TilesMessage> + Send + 'static;
}

