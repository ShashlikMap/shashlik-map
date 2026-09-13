use osm::tiles::TileKey;

pub mod default_tiles_provider;
pub mod mvt;
pub mod shashlik;
pub mod shashlik_v1;
pub mod tile_data;
mod tile_parser;
pub mod tiles_provider;

pub struct CustomTileKey<'a>(&'a TileKey);

impl<'a> CustomTileKey<'a> {
    fn get_tile_x(&self) -> i32 {
        let max_tile = 1 << self.0.zoom_level;
        let tile_x = self.0.tile_x;
        tile_x.rem_euclid(max_tile)
    }

    fn get_tile_y(&self) -> i32 {
        self.0.tile_y
    }

    fn get_zoom_level(&self) -> i32 {
        self.0.zoom_level
    }
}
