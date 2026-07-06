use crate::tiles::mvt::mvt_scheme_parser::MvtSchemeParser;
use crate::MAX_ZOOM_LEVEL;
use fast_mvt::proto::GeomType;
use fast_mvt::{MvtReaderRef, MvtResult};
use geo::MapCoords;
use geo_types::{Coord, Geometry, LineString, Polygon};
use osm::map::{MapGeomObject, MapGeometry};
use osm::tiles::TileKey;

pub struct MvtParser {
    schema_parser: MvtSchemeParser,
}

impl Default for MvtParser {
    fn default() -> Self {
        MvtParser::new()
    }
}

impl MvtParser {
    pub fn new() -> Self {
        Self {
            schema_parser: MvtSchemeParser::new_map_tiler_v4(),
        }
    }
    fn get_all_lines(geometry: Geometry<i32>) -> Vec<LineString<i32>> {
        match geometry {
            Geometry::LineString(line_string) => {
                vec![line_string]
            }
            Geometry::MultiLineString(multi_line_string) => multi_line_string.into_iter().collect(),
            _ => Vec::new(),
        }
    }

    fn get_all_polygons(geometry: Geometry<i32>) -> Vec<Polygon<i32>> {
        match geometry {
            Geometry::Polygon(polygon) => {
                vec![polygon]
            }
            Geometry::MultiPolygon(multi_polygon) => multi_polygon.into_iter().collect(),
            _ => Vec::new(),
        }
    }

    pub fn read_mvt_tile(
        &self,
        bytes: &[u8],
        tile_key: &TileKey,
    ) -> MvtResult<Vec<(MapGeomObject, MapGeometry<f32>)>> {
        let reader = MvtReaderRef::new(bytes)?;
        let mut result = self.schema_parser.parse(reader.layers(), |feature| {
            let mut res = vec![];
            let geom_type = feature.geom_type();
            let geometry = feature.geometry();

            if let (Some(geom_type), Some(geometry)) = (geom_type, geometry.ok()) {
                // TODO Add points later
                if geom_type == GeomType::LINESTRING {
                    for line in Self::get_all_lines(geometry) {
                        res.push(MapGeometry::Line(line));
                    }
                } else if geom_type == GeomType::POLYGON {
                    for polygon in Self::get_all_polygons(geometry) {
                        res.push(MapGeometry::Poly(polygon));
                    }
                }
            }
            res
        });

        result.sort_by(|(a, _), (b, _)| a.cmp(b));
        let fixed = result
            .into_iter()
            .map(|(geom, obj)| {
                let obj_fixed = Self::convert_and_restore_data(&obj, tile_key);
                (geom, obj_fixed)
            })
            .collect();

        Ok(fixed)
    }

    fn convert_and_restore_data(
        geometry: &MapGeometry<i32>,
        tile_key: &TileKey,
    ) -> MapGeometry<f32> {
        // FIXME Zoom level handling
        let factor = 2.0f32.powf((MAX_ZOOM_LEVEL - tile_key.zoom_level) as f32);
        let koef = 512.0f32 / 4096.0;
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
