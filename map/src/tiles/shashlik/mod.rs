use crate::tiles::tiles_provider::{MercatorConverter, MercatorProvider, TilesProviderStore};
use crate::MAX_ZOOM_LEVEL;
use geo::{BoundingRect, Intersects, MapCoordsInPlace, Scale};
use geo_types::{Coord, Polygon, Rect};
use glam::DVec3;
use osm::map::{MapGeomObject, MapGeometry};
use osm::source::TileSource;
use osm::tiles::{calc_tile_ranges, TileKey, TileStore, TILE_SIZE, TILES_COUNT, TILE_OVERLAP_PERCENT};

impl<S: TileSource> MercatorProvider<TILE_SIZE> for TileStore<S> {}

/// TileStore uses hardcoded zoom 22, so the caller's zoom has to be ignored
impl<S: TileSource> MercatorConverter for TileStore<S> {
    fn lon_lat_to_world(&self, lon_lat: &Coord<f64>, _zoom_level: i32) -> Coord<f64> {
        let lon_lat: (f64, f64) = (*lon_lat).into();
        Self::mercator()
            .from_ll_to_subpixel(&lon_lat, 22)
            .unwrap()
            .into()
    }

    fn world_to_lon_lat(&self, xy: &Coord<f64>, _zoom_level: i32) -> Coord<f64> {
        let xy: (f64, f64) = (*xy).into();
        Self::mercator()
            .from_pixel_to_ll(&xy, 22)
            .unwrap()
            .into()
    }
}
impl <S:TileSource> TilesProviderStore for TileStore<S> {
    fn convert_zoom(&self, zoom_level: i32) -> i32 {
        MAX_ZOOM_LEVEL - zoom_level
    }

    fn tile_ranges(&self, mut area: Polygon<f64>, zoom_level: i32) -> Vec<TileKey> {
        let zoom_level = self.convert_zoom(zoom_level);

        area.map_coords_in_place(|coord| {
            self.world_to_lon_lat(&coord, zoom_level)
        });

        // this will be compared for intersection later, it should have a correct winding
        let area_lon_lat = area.exterior().bounding_rect().unwrap();

        let ranges = calc_tile_ranges(TILES_COUNT, zoom_level, &area_lon_lat);
        let mut res = vec![];
        for tx in ranges.min_x..=ranges.max_x {
            for ty in ranges.min_y..=ranges.max_y {
                let tile_key = TileKey {
                    tile_x: tx as i32,
                    tile_y: ty as i32,
                    zoom_level,
                };

                // FIXME Maybe move "calc_tile_boundary" to tile generator? since we need to calculate all the time and twice(+ before loading)
                let tile_rect = tile_key.calc_tile_boundary(1.0);
                if area.intersects(&tile_rect) {
                    res.push(tile_key);
                }
            }
        }
        res
    }

    fn tile_position_bbox(&self, tile_key: &TileKey, bbox_scale: f64) -> (DVec3, Rect) {
        let tile_rect = tile_key.calc_tile_boundary(TILE_OVERLAP_PERCENT);

        let tile_rect_origin = self.lon_lat_to_world(&tile_rect.min(), MAX_ZOOM_LEVEL);
        let tile_position = [tile_rect_origin.x, tile_rect_origin.y, 0.0].into();

        let tile_rect_original = tile_key.calc_tile_boundary(1.00);
        let tile_rect_original_min = self.lon_lat_to_world(&tile_rect_original.min(), MAX_ZOOM_LEVEL);
        let tile_rect_original_max = self.lon_lat_to_world(&tile_rect_original.max(), MAX_ZOOM_LEVEL);
        let bbox = Rect::new(tile_rect_original_min, tile_rect_original_max).scale(bbox_scale);
        (tile_position, bbox)
    }

    fn load(&self, tile_key: &TileKey) -> Vec<(MapGeomObject, MapGeometry<f32>)> {
        self.load_geometries(tile_key)
    }
}