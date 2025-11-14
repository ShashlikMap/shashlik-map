use std::io::Read;
use flate2::read::GzDecoder;
use crate::map::{get_world_boundary, MapGeomObject, MapGeometry, MapGeometryCollection};
use crate::source::TileSource;
use geo::{coord, Rect, Scale};
use serde::{Deserialize, Serialize};

pub const TILES_COUNT: i32 = 32768;

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TileKey {
    pub tile_x: i32,
    pub tile_y: i32,
    pub zoom_level: i32,
}

impl TileKey {
    pub fn as_string_key(&self) -> String {
        format!("({}, {}, {})", self.tile_x, self.tile_y, self.zoom_level)
    }

    pub fn calc_tile_boundary(&self, scale_factor: f64) -> Rect {
        let world_rect = get_world_boundary();

        let tiles_count = (TILES_COUNT / 2i32.pow(self.zoom_level as u32)).max(1);
        let tile_width = world_rect.width() / tiles_count as f64;
        let tile_height = world_rect.height() / tiles_count as f64;
        let p1 = coord!(x: tile_width * self.tile_x as f64 + world_rect.min().x,
            y: tile_height * self.tile_y as f64 + world_rect.min().y);
        let p2 = coord!(x: p1.x + tile_width, y: p1.y + tile_height);
        Rect::new(p1, p2).scale(scale_factor)
    }
}

#[derive(Clone, Debug)]
pub struct TileRanges {
    pub min_x: u32,
    pub max_x: u32,
    pub min_y: u32,
    pub max_y: u32,
}
pub fn calc_tile_ranges(total_tiles: i32, zoom_level: i32, rect: &Rect) -> TileRanges {
    let world_rect = get_world_boundary();
    let tiles_count = (total_tiles / 2i32.pow(zoom_level as u32)).max(1);
    let tiles_count_f64 = tiles_count as f64;

    let tile_min_x =
        ((tiles_count_f64 * ((rect.min().x - world_rect.min().x) / world_rect.width())) as i32)
            .clamp(0, tiles_count - 1);
    let tile_max_x =
        ((tiles_count_f64 * ((rect.max().x - world_rect.min().x) / world_rect.width())) as i32)
            .clamp(0, tiles_count - 1);
    let tile_min_y =
        ((tiles_count_f64 * ((rect.min().y - world_rect.min().y) / world_rect.height())) as i32)
            .clamp(0, tiles_count - 1);
    let tile_max_y =
        ((tiles_count_f64 * ((rect.max().y - world_rect.min().y) / world_rect.height())) as i32)
            .clamp(0, tiles_count - 1);

    TileRanges {
        min_x: tile_min_x as u32,
        max_x: tile_max_x as u32,
        min_y: tile_min_y as u32,
        max_y: tile_max_y as u32,
    }
}

pub fn closest_power_2(zoom: i32) -> i32 {
    if zoom < 2 {
        return zoom;
    }
    let mut t = 1;
    let mut a = 0;
    // TODO change to log
    while t * 2 <= zoom {
        t *= 2;
        a += 1;
    }
    a + 1
}

impl TileKey {
    pub fn new(tile_x: i32, tile_y: i32, zoom_level: i32) -> Self {
        TileKey {
            tile_x,
            tile_y,
            zoom_level,
        }
    }
}

pub struct TileStore<S: TileSource> {
    tile_source: S,
}

impl<S: TileSource> TileStore<S> {
    pub fn new(tile_source: S) -> TileStore<S> {
        Self { tile_source }
    }

    // TODO Report
    pub fn load_geometries(&self, tile_key: &TileKey) -> Vec<(MapGeomObject, MapGeometry)> {
        let data = self
            .tile_source
            .fetch(tile_key.tile_x, tile_key.tile_y, tile_key.zoom_level)
            .unwrap_or_else(|err| {
                println!("Failed to fetch tile key {tile_key:?}. Error: {err}");
                vec![]
            });
        let mut decoder = GzDecoder::new(&data[..]);
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data).unwrap_or_else(|err| {
            println!("Failed to decompress tile key {tile_key:?}. Error: {err}");
            0
        });
        let collection: MapGeometryCollection = bincode::deserialize(&decompressed_data).unwrap_or_else(|err| {
            println!("Failed to deserialize tile key {tile_key:?}, Error: {err}");
            MapGeometryCollection(vec![])
        });
        collection.0
    }
}
