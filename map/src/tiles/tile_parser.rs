use crate::MAX_ZOOM_LEVEL;
use geo::MapCoords;
use geo_types::Coord;
use osm::map::{MapGeomObject, MapGeometry};
use osm::tiles::TileKey;

pub(crate) trait TileParser<T> {
    fn parse_tile_inner(&self, data: T) -> Vec<(MapGeomObject, MapGeometry<i32>)>;

    fn parse_tile(
        &self,
        data: T,
        extent: f32,
        tile_key: &TileKey,
    ) -> Vec<(MapGeomObject, MapGeometry<f32>)> {
        let mut result = self.parse_tile_inner(data);
        result.sort_by(|(a, _), (b, _)| a.cmp(b));
        let fixed = result
            .into_iter()
            .map(|(geom, obj)| {
                let obj_fixed = Self::convert_and_restore_data(&obj, extent, tile_key);
                (geom, obj_fixed)
            })
            .collect();

        fixed
    }

    fn convert_and_restore_data(
        geometry: &MapGeometry<i32>,
        extent: f32,
        tile_key: &TileKey,
    ) -> MapGeometry<f32> {
        // FIXME Zoom level handling
        let factor = 2.0f32.powf((MAX_ZOOM_LEVEL - tile_key.zoom_level) as f32);
        let koef = 512.0f32 / extent;
        let total_multiplier = factor * koef;
        let convert_coord = |c: &Coord<i32>| -> Coord<f32> {
            Coord {
                x: (c.x as f32) * total_multiplier,
                y: (c.y as f32) * total_multiplier,
            }
        };
        match geometry {
            MapGeometry::Line(line) => MapGeometry::Line(line.map_coords(|c| convert_coord(&c))),
            MapGeometry::Poly(poly) => MapGeometry::Poly(poly.map_coords(|c| convert_coord(&c))),
            MapGeometry::Coord(coord) => MapGeometry::Coord(convert_coord(coord)),
        }
    }
}
