use crate::MAX_ZOOM_LEVEL;
use geo::MapCoords;
use geo_types::{Coord, LineString, Polygon, coord};
use osm::map::{
    HighwayKind, LayerKind, LineKind, MapGeomObject, MapGeomObjectKind, MapGeometry, MapPointInfo,
    MapPointObjectKind, NatureKind, PopAreaInfo, RailwayKind, WayInfo,
};
use osm::tiles::TileKey;
use tiles::decode::{AreaKind, DecodedTile, LabelClass, RoadKind};

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

    pub fn read_decoded_tile(
        &self,
        tile: DecodedTile,
        tile_key: &TileKey,
    ) -> Vec<(MapGeomObject, MapGeometry<f32>)> {
        let mut result = vec![];
        for road in tile.roads {
            let line_kind = match road.kind {
                RoadKind::Motorway | RoadKind::MajorRoad => LineKind::Highway {
                    kind: HighwayKind::Motorway,
                },
                RoadKind::Trunk => LineKind::Highway {
                    kind: HighwayKind::Trunk,
                },
                RoadKind::Primary => LineKind::Highway {
                    kind: HighwayKind::Primary,
                },
                RoadKind::Secondary => LineKind::Highway {
                    kind: HighwayKind::Secondary,
                },
                RoadKind::Tertiary => LineKind::Highway {
                    kind: HighwayKind::Tertiary,
                },
                RoadKind::Unclassified => LineKind::Highway {
                    kind: HighwayKind::Unclassified,
                },
                RoadKind::Residential => LineKind::Highway {
                    kind: HighwayKind::Residential,
                },
                RoadKind::LivingStreet => LineKind::Highway {
                    kind: HighwayKind::Residential,
                },
                RoadKind::Service => LineKind::Highway {
                    kind: HighwayKind::Service,
                },
                RoadKind::Unknown => LineKind::Highway {
                    kind: HighwayKind::Unclassified,
                },
                RoadKind::Rail => LineKind::Railway {
                    kind: RailwayKind::Rail,
                },
                _ => continue,
            };

            let map_geom_obj = MapGeomObject {
                id: -1,
                kind: MapGeomObjectKind::Way(WayInfo {
                    line_kind,
                    layer: road.layer as i32,
                    layer_kind: LayerKind::None,
                    name_en: road.name,
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

        for area in tile.areas {
            let area_kind = match area.kind {
                AreaKind::Water => NatureKind::Water,
                AreaKind::Forest => NatureKind::Forest,
                AreaKind::Grass => NatureKind::Park,
                AreaKind::Building => continue,
                AreaKind::Land => continue,
            };

            let map_geom_obj = MapGeomObject {
                id: -1,
                kind: MapGeomObjectKind::Nature(area_kind),
            };

            let qgg = area.rings[0]
                .iter()
                .map(|c| {
                    coord! {x: c[0] as i32, y: c[1] as i32 }
                })
                .collect();

            let hh = MapGeometry::Poly(Polygon::new(LineString::new(qgg), vec![]));
            result.push((map_geom_obj, hh))
        }

        for label in tile.labels {
            let _ = match label.class {
                LabelClass::City => {}
                _ => continue,
            };

            let map_geom_obj = MapGeomObject {
                id: -1,
                kind: MapGeomObjectKind::Poi(MapPointInfo {
                    text: label.name,
                    kind: MapPointObjectKind::PopArea(PopAreaInfo {
                        level: 0,
                        population: 0,
                    }),
                }),
            };
            let hh =
                MapGeometry::Coord(coord! { x: label.anchor[0] as i32, y: label.anchor[1] as i32 });
            result.push((map_geom_obj, hh))
        }

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
