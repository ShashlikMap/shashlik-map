use crate::tiles::tile_data::TileData;
use futures::Stream;
use geo_types::{coord, Coord, Polygon, Rect};
use glam::DVec3;
use googleprojection::Mercator;
use osm::map::{MapGeomObject, MapGeometry};
use osm::tiles::{TileKey, TileRanges};
use std::collections::HashSet;
use std::sync::Arc;
use geo::{CoordsIter, Scale};

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

pub struct TileMetersBounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

const EXTENT: f64 = 8388608.0;
const MAP_SIZE: f64 = EXTENT * 2.0;

pub trait TilesProviderStore: MercatorConverter {

    fn convert_zoom(&self, zoom_level: i32) -> i32 {
        zoom_level
    }
    fn tile_ranges(&self, area: geo_types::Polygon<f64>, zoom_level: i32) -> Vec<TileKey> {
        let mut min_x = u32::MAX;
        let mut max_x = u32::MIN;
        let mut min_y = u32::MAX;
        let mut max_y = u32::MIN;

        for coord in area.coords_iter() {
            let (tx, ty) = self.mercator_meters_to_512_tile(coord.x, coord.y, zoom_level as u32);

            if tx < min_x { min_x = tx; }
            if tx > max_x { max_x = tx; }
            if ty < min_y { min_y = ty; }
            if ty > max_y { max_y = ty; }
        }


        let ranges = TileRanges {
            min_x,
            max_x,
            min_y,
            max_y,
        };

        let mut res = vec![];
        for tx in ranges.min_x..=ranges.max_x {
            for ty in ranges.min_y..=ranges.max_y {
                let tile_key = TileKey {
                    tile_x: tx as i32,
                    tile_y: ty as i32,
                    zoom_level,
                };

                res.push(tile_key);

                // TODO check intersection!
                // // FIXME Maybe move "calc_tile_boundary" to tile generator? since we need to calculate all the time and twice(+ before loading)
                // let tile_rect = tile_key.calc_tile_boundary(1.0);
                // if area.intersects(&tile_rect) {
                //     res.push(tile_key);
                // }
            }
        }
        res
    }
    fn tile_position_bbox(&self, tile_key: &TileKey, bbox_scale: f64) -> (DVec3, Rect) {
        let bounds = self.tile_id_to_mercator_meters(
            tile_key.tile_x as u32,
            tile_key.tile_y as u32,
            tile_key.zoom_level as u32,
        );
        let tile_position: DVec3 = DVec3::new(bounds.min_x, bounds.min_y, 0.0);

        let bbox = Rect::new(
            coord! {x: bounds.min_x, y: bounds.min_y},
            coord! {x: bounds.max_x, y: bounds.max_y},
        )
            .scale(bbox_scale);

        (tile_position, bbox)
    }
    fn load(&self, tile_key: &TileKey) -> Vec<(MapGeomObject, MapGeometry<f32>)>;

    fn mercator_meters_to_512_tile(&self, mx: f64, my: f64, zoom: u32) -> (u32, u32) {
        let norm_x = (mx) / MAP_SIZE;
        let norm_y = (my) / MAP_SIZE; // Flipped because Tile Y increases downwards

        let num_tiles = (1 << zoom) as f64;

        let tx = (norm_x * num_tiles).floor() as u32;
        let ty = (norm_y * num_tiles).floor() as u32;

        let max_tile = (1 << zoom) - 1;
        (tx.min(max_tile), ty.min(max_tile))
    }

    fn tile_id_to_mercator_meters(&self, tx: u32, ty: u32, zoom: u32) -> TileMetersBounds {
        let num_tiles = (1 << zoom) as f64;

        let norm_left = tx as f64 / num_tiles;
        let norm_right = (tx + 1) as f64 / num_tiles;

        let norm_top = ty as f64 / num_tiles;
        let norm_bottom = (ty + 1) as f64 / num_tiles;

        let min_x = norm_left * MAP_SIZE;
        let max_x = norm_right * MAP_SIZE;

        let max_y = norm_bottom * MAP_SIZE;
        let min_y = norm_top * MAP_SIZE;

        TileMetersBounds {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }
}

pub trait TilesProvider: MercatorConverter {
    fn inner_converter(&self) -> Arc<dyn MercatorConverter>;
    fn load(&mut self, area_poly: Polygon<f64>, zoom_level: i32);

    fn tiles(&mut self) -> impl Stream<Item=TilesMessage> + Send + 'static;
}

