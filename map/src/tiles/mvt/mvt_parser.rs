use crate::tiles::mvt::mvt_scheme_parser::MvtSchemeParser;
use crate::tiles::tile_parser::TileParser;
use fast_mvt::proto::GeomType;
use fast_mvt::{MvtReaderRef, MvtResult};
use geo_types::{Geometry, LineString, Point, Polygon, coord};
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

    fn get_all_points(geometry: Geometry<i32>) -> Vec<Point<i32>> {
        match geometry {
            Geometry::Point(point) => {
                vec![point]
            }
            Geometry::MultiPoint(multi_point) => multi_point.into_iter().collect(),
            _ => Vec::new(),
        }
    }

    pub fn read_mvt_tile(
        &self,
        bytes: &[u8],
        tile_key: &TileKey,
    ) -> MvtResult<Vec<(MapGeomObject, MapGeometry<f32>)>> {
        Ok(self.parse_tile(bytes, 4096.0, tile_key))
    }
}

impl TileParser<&[u8]> for MvtParser {
    fn parse_tile_inner(&self, data: &[u8]) -> Vec<(MapGeomObject, MapGeometry<i32>)> {
        let mut result = vec![];
        if let Ok(reader) = MvtReaderRef::new(data) {
            result = self.schema_parser.parse(reader.layers(), |feature| {
                let mut res = vec![];
                let geom_type = feature.geom_type();
                let geometry = feature.geometry();

                if let (Some(geom_type), Some(geometry)) = (geom_type, geometry.ok()) {
                    if geom_type == GeomType::LINESTRING {
                        for line in Self::get_all_lines(geometry) {
                            res.push(MapGeometry::Line(line));
                        }
                    } else if geom_type == GeomType::POLYGON {
                        for polygon in Self::get_all_polygons(geometry) {
                            res.push(MapGeometry::Poly(polygon));
                        }
                    } else if geom_type == GeomType::POINT {
                        for point in Self::get_all_points(geometry) {
                            res.push(MapGeometry::Coord(coord! {x: point.x(), y: point.y()}));
                        }
                    }
                }
                res
            })
        }
        result
    }
}
