use crate::tiles::tile_data::TileData;
use futures::Stream;
use geo_types::{Coord, Polygon, Rect};
use glam::DVec3;
use googleprojection::Mercator;
use osm::map::{MapGeomObject, MapGeometry};
use osm::tiles::TileKey;
use std::collections::HashSet;
use std::sync::Arc;

pub enum TilesMessage {
    TilesData(Vec<TileData>),
    ToRemove(HashSet<String>),
}

pub trait MercatorProvider {
    fn mercator(&self) -> Mercator {
        Mercator::with_size(512)
    }
}

pub trait MercatorConverter: Send + Sync + MercatorProvider{
    fn lon_lat_to_world(&self, lon_lat: &Coord<f64>, zoom_level: i32) -> Coord<f64> {
        let lon_lat: (f64, f64) = (*lon_lat).into();

        self.mercator()
            .from_ll_to_subpixel(&lon_lat, zoom_level as usize)
            .unwrap()
            .into()
    }
    fn world_to_lon_lat(&self, xy: &Coord<f64>, zoom_level: i32) -> Coord<f64> {
        let xy: (f64, f64) = (*xy).into();
        self.mercator()
            .from_pixel_to_ll(&xy, zoom_level as usize)
            .unwrap()
            .into()
    }
}

pub trait TilesProviderStore: MercatorConverter {
    fn convert_zoom(&self, zoom_level: i32) -> i32;
    fn tile_ranges(&self, area: geo_types::Polygon<f64>, zoom_level: i32) -> Vec<TileKey>;
    fn tile_position_bbox(&self, tile_key: &TileKey, bbox_scale: f64) -> (DVec3, Rect);
    fn load(&self, tile_key: &TileKey) -> Vec<(MapGeomObject, MapGeometry<f32>)>;
}

pub trait TilesProvider: MercatorConverter {
    fn inner_converter(&self) -> Arc<dyn MercatorConverter>;
    fn load(&mut self, area_poly: Polygon<f64>, zoom_level: i32);

    fn tiles(&mut self) -> impl Stream<Item=TilesMessage> + Send + 'static;
}

