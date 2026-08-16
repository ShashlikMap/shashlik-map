use crate::MAX_ZOOM_LEVEL;
use geo::MapCoords;
use geo_types::{Coord, Geometry, LineString, Point, Polygon, coord};
use osm::map::{
    HighwayKind, LayerKind, LineKind, MapGeomObject, MapGeomObjectKind, MapGeometry, WayInfo,
};
use osm::tiles::TileKey;
use tiles::decode::{DecodedTile, RoadKind};

pub struct ShashlikV1Parser {}

impl Default for ShashlikV1Parser {
    fn default() -> Self {
        ShashlikV1Parser::new()
    }
}

impl ShashlikV1Parser {
    pub fn new() -> Self {
        Self {}
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

    pub fn read_decoded_tile(
        &self,
        tile: DecodedTile,
        tile_key: &TileKey,
    ) -> Vec<(MapGeomObject, MapGeometry<f32>)> {
        let mut result = vec![];
        println!("tile r: {:?}, ",tile.roads.len());
        println!("tile a: {:?}, ",tile.areas.len());
        println!("tile extent: {:?}, ",tile.extent);
        println!("tile labels: {:?}, ",tile.labels.len());
        println!("tile pois: {:?}, ",tile.pois.len());
        for road in tile.roads {

            let line_kind = match road.kind {
                RoadKind::Motorway => HighwayKind::Motorway,
                RoadKind::Trunk => HighwayKind::Trunk,
                RoadKind::Primary => HighwayKind::Primary,
                RoadKind::Secondary => HighwayKind::Secondary,
                RoadKind::Tertiary => HighwayKind::Tertiary,
                RoadKind::Unclassified => HighwayKind::Unclassified,
                RoadKind::Residential => HighwayKind::Residential,
                RoadKind::LivingStreet => HighwayKind::Residential,
                RoadKind::Service => HighwayKind::Service,
                _ => continue,
            };

            let map_geom_obj = MapGeomObject {
                id: -1,
                kind: MapGeomObjectKind::Way(WayInfo {
                    line_kind: LineKind::Highway { kind: line_kind },
                    layer: road.layer as i32,
                    layer_kind: LayerKind::None,
                    name_en: None,
                }),
            };

            let qgg = road
                .coords
                .iter()
                .map(|c| {
                    coord! {x: c[0] as i32, y: c[1] as i32 }
                })
                .collect();
            let hh = MapGeometry::Line(LineString::new(qgg));
            result.push((map_geom_obj, hh))
        }
        // let mut result = self.schema_parser.parse(reader.layers(), |feature| {
        //     let mut res = vec![];
        //     let geom_type = feature.geom_type();
        //     let geometry = feature.geometry();
        //
        //     if let (Some(geom_type), Some(geometry)) = (geom_type, geometry.ok()) {
        //         if geom_type == GeomType::LINESTRING {
        //             for line in Self::get_all_lines(geometry) {
        //                 res.push(MapGeometry::Line(line));
        //             }
        //         } else if geom_type == GeomType::POLYGON {
        //             for polygon in Self::get_all_polygons(geometry) {
        //                 res.push(MapGeometry::Poly(polygon));
        //             }
        //         } else if geom_type == GeomType::POINT {
        //             for point in Self::get_all_points(geometry) {
        //                 res.push(MapGeometry::Coord(coord! {x: point.x(), y: point.y()}));
        //             }
        //         }
        //     }
        //     res
        // });

        result.sort_by(|(a, _), (b, _)| a.cmp(b));
        let fixed = result
            .into_iter()
            .map(|(geom, obj)| {
                let obj_fixed = Self::convert_and_restore_data(&obj, tile_key);
                (geom, obj_fixed)
            })
            .collect();

        fixed
    }

    fn convert_and_restore_data(
        geometry: &MapGeometry<i32>,
        tile_key: &TileKey,
    ) -> MapGeometry<f32> {
        // FIXME Zoom level handling
        let factor = 2.0f32.powf((MAX_ZOOM_LEVEL - tile_key.zoom_level) as f32);
        let koef = 512.0f32 / 8192.0;
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
