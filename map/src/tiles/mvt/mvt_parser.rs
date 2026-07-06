use crate::MAX_ZOOM_LEVEL;
use crate::tiles::mvt::mvt_scheme_parser::MvtSchemeParser;
use fast_mvt::proto::GeomType;
use fast_mvt::{MvtGeometry, MvtReaderRef, MvtResult, MvtValue};
use geo::{CoordsIter, MapCoords};
use geo_types::{Geometry, LineString, Polygon, coord};
use log::error;
use osm::map::{MapGeomObject, MapGeometry};
use osm::tiles::TileKey;

pub struct MvtParser {
    mvt_s: MvtSchemeParser,
}

// TODO This is WIP and requires reworking. So far just a temporary PoC implementation
impl MvtParser {
    pub fn new() -> Self {
        Self {
            mvt_s: MvtSchemeParser::new_map_tiler_v4(),
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

    pub fn read_mvt_tile(
        &self,
        bytes: &[u8],
        tile_key: &TileKey,
    ) -> MvtResult<Vec<(MapGeomObject, MapGeometry<f32>)>> {
        let reader = MvtReaderRef::new(bytes)?;
        let koef = 512.0f32 / 4096.0;
        let mut result = self.mvt_s.parse(reader.layers(), |feature| {
            let mut res = vec![];
            let geom_type = feature.geom_type();
            let geometry = feature.geometry();

            if let Some(geom_type) = geom_type
                && geometry.is_ok()
            {
                let geometry = geometry.unwrap();
                if geom_type == GeomType::LINESTRING {
                    for line in Self::get_all_lines(geometry) {
                        let ls: LineString<f32> = line
                            .coords_iter()
                            .map(|coord| {
                                coord! { x: coord.x as f32 * koef, y: coord.y as f32 * koef}
                            })
                            .collect();

                        res.push(MapGeometry::Line(ls));
                    }
                } else if geom_type == GeomType::POLYGON {
                    match geometry {
                        MvtGeometry::Polygon(poly) => {
                            let ls: Polygon<f32> = poly.map_coords(|coord| {
                                coord! { x: coord.x as f32 * koef, y: coord.y as f32 * koef}
                            });
                            res.push(MapGeometry::Poly(ls));
                        }
                        MvtGeometry::MultiPolygon(polis) => {
                            for poly in polis {
                                let ls: Polygon<f32> = poly.map_coords(|coord| {
                                    coord! { x: coord.x as f32 * koef, y: coord.y as f32 * koef}
                                });
                                res.push(MapGeometry::Poly(ls));
                            }
                        }
                        _ => {}
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
        geometry: &MapGeometry<f32>,
        tile_key: &TileKey,
    ) -> MapGeometry<f32> {
        // FIXME Zoom level handling
        let factor = 2.0f32.powf((MAX_ZOOM_LEVEL - tile_key.zoom_level) as f32);
        match geometry {
            MapGeometry::Line(line) => MapGeometry::Line(line.map_coords(|coord| coord * factor)),
            MapGeometry::Poly(poly) => MapGeometry::Poly(poly.map_coords(|coord| coord * factor)),
            MapGeometry::Coord(coord) => MapGeometry::Coord(*coord * factor),
        }
    }
}

pub(crate) struct LocalMvtValue(pub MvtValue);

impl LocalMvtValue {
    fn unexpected_type<T>(&self, expected: &str) -> Option<T> {
        error!("Unexpected {} MvtValueRef: {:?}", expected, self.0);
        None
    }
}
impl From<LocalMvtValue> for Option<i64> {
    fn from(value: LocalMvtValue) -> Self {
        match value.0 {
            MvtValue::SInt(value) => Some(value),
            _ => value.unexpected_type("i64"),
        }
    }
}

impl From<LocalMvtValue> for Option<String> {
    fn from(value: LocalMvtValue) -> Self {
        match value.0 {
            MvtValue::String(value) => Some(value),
            _ => value.unexpected_type("String"),
        }
    }
}

impl From<LocalMvtValue> for Option<bool> {
    fn from(value: LocalMvtValue) -> Self {
        match value.0 {
            MvtValue::Bool(value) => Some(value),
            _ => value.unexpected_type("bool"),
        }
    }
}
