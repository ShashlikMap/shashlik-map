use crate::tiles::parsers::tile_parser::TileParser;
use crate::tiles::{
    ShashlikMapGeomObject, ShashlikMapGeomObjectKind, ShashlikMapPointInfo,
    ShashlikMapPointObjectKind, ShashlikNatureKind, ShashlikPopAreaInfo, ShashlikWayInfo,
};
use geo_types::{LineString, Polygon, coord};
use osm::map::{HighwayKind, LayerKind, LineKind, MapGeometry, RailwayKind};
use osm::tiles::TileKey;
use tiles::decode::{AreaKind, DecodedTile, LabelClass, PoiKind, RoadKind, RoadStructure};

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
    ) -> Vec<(ShashlikMapGeomObject, MapGeometry<f32>)> {
        let extent = tile.extent;
        self.parse_tile(tile, extent as f32, tile_key)
    }
}

impl TileParser<DecodedTile> for ShashlikV1Parser {
    fn parse_tile_inner(
        &self,
        tile: DecodedTile,
    ) -> Vec<(ShashlikMapGeomObject, MapGeometry<i32>)> {
        let mut result = vec![];
        for road in tile.roads {
            let layer_kind = match road.structure {
                RoadStructure::None => LayerKind::None,
                RoadStructure::Bridge => LayerKind::Bridge,
                RoadStructure::Tunnel => LayerKind::Tunnel,
            };
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

            let map_geom_obj = ShashlikMapGeomObject {
                id: -1,
                kind: ShashlikMapGeomObjectKind::Way(ShashlikWayInfo {
                    line_kind,
                    layer: road.layer as i32,
                    layer_kind,
                    name_en: road.name,
                    linear_refs: vec![],
                }),
            };

            let coords: Vec<_> = road
                .coords
                .iter()
                .map(|c| {
                    coord! {x: c[0] as i32, y: c[1] as i32 }
                })
                .collect();
            let line = MapGeometry::Line(coords.into());
            result.push((map_geom_obj, line))
        }

        for area in tile.areas {
            let obj = match area.kind {
                AreaKind::Water => ShashlikMapGeomObjectKind::Nature(ShashlikNatureKind::Water),
                AreaKind::Forest => ShashlikMapGeomObjectKind::Nature(ShashlikNatureKind::Forest),
                AreaKind::Grass => ShashlikMapGeomObjectKind::Nature(ShashlikNatureKind::Park),
                AreaKind::Building => ShashlikMapGeomObjectKind::Building(area.floors as u16),
                AreaKind::Land => continue,
            };

            let map_geom_obj = ShashlikMapGeomObject { id: -1, kind: obj };

            let mut rings: Vec<LineString<i32>> = area
                .rings
                .iter()
                .map(|ring| {
                    let coords = ring
                        .iter()
                        .map(|c| {
                            coord! {x: c[0] as i32, y: c[1] as i32 }
                        })
                        .collect::<Vec<_>>();
                    LineString::<i32>(coords)
                })
                .collect();
            if !rings.is_empty() {
                let poly = MapGeometry::Poly(Polygon::new(rings.remove(0), rings));
                result.push((map_geom_obj, poly))
            }
        }

        for label in tile.labels {
            let _ = match label.class {
                LabelClass::City => {}
                _ => continue,
            };

            let map_geom_obj = ShashlikMapGeomObject {
                id: 0,
                kind: ShashlikMapGeomObjectKind::Poi(ShashlikMapPointInfo {
                    text: label.name,
                    kind: ShashlikMapPointObjectKind::PopArea(ShashlikPopAreaInfo {
                        level: 0,
                        population: 0,
                    }),
                }),
            };
            let coord =
                MapGeometry::Coord(coord! { x: label.anchor[0] as i32, y: label.anchor[1] as i32 });
            result.push((map_geom_obj, coord))
        }

        for poi in tile.pois {
            match poi.kind {
                PoiKind::TrafficSignal => {}
            };

            let map_geom_obj = ShashlikMapGeomObject {
                id: 0,
                kind: ShashlikMapGeomObjectKind::Poi(ShashlikMapPointInfo {
                    text: "".to_string(),
                    kind: ShashlikMapPointObjectKind::TrafficLight,
                }),
            };
            let coord =
                MapGeometry::Coord(coord! { x: poi.anchor[0] as i32, y: poi.anchor[1] as i32 });
            result.push((map_geom_obj, coord))
        }
        result
    }
}
