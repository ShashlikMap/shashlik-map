use std::fmt::format;
use crate::MAX_ZOOM_LEVEL;
use fast_mvt::proto::GeomType;
use fast_mvt::{MvtGeometry, MvtReaderRef, MvtResult, MvtValueRef};
use geo::{CoordsIter, MapCoords};
use geo_types::{coord, Geometry, LineString, Polygon};
use osm::map::{
    HighwayKind, LayerKind, LineKind, MapGeomObject, MapGeomObjectKind, MapGeometry, NatureKind,
    WayInfo,
};
use osm::tiles::TileKey;

pub struct MvtParser;

// TODO This is WIP and requires reworking. So far just a temporary PoC implementation
impl MvtParser {
    fn get_all_lines(geometry: Geometry<i32>) -> Vec<LineString<i32>> {
        match geometry {
            Geometry::LineString(line_string) => {
                // .lines() returns an iterator over individual Line segments
                vec![line_string]
            }
            Geometry::MultiLineString(multi_line_string) => {
                // MultiLineString contains multiple LineStrings;
                // iterate over them, get lines for each, and flatten
                multi_line_string
                    .into_iter()
                    .collect()
            }
            _ => {
                // Handle or ignore other geometry variants (like Point, Polygon, etc.)
                Vec::new()
            }
        }
    }

    pub fn read_mvt_tile(
        bytes: &[u8],
        tile_key: &TileKey,
    ) -> MvtResult<Vec<(MapGeomObject, MapGeometry<f32>)>> {
        let mut res = vec![];
        let reader = MvtReaderRef::new(bytes)?;

        for layer in reader.layers() {
            // println!("layer: {:?}",layer.name());
            if layer.name() == "road" {
                for feature in layer.features() {
                    if let Some(geom_type) = feature.geom_type()
                        && geom_type == GeomType::LINESTRING
                    {
                        let mut layer: i64 = 0;
                        let mut highway_kind_name: Option<&str> = None;
                        let mut brunnel = false;
                        let mut ramp = false;
                        for property in feature.properties() {
                            let (key, value) = property?;
                            if key == "layer" {
                                layer = LocalMvtValue(value).into();
                            } else if key == "class" {
                                let road_class: String = LocalMvtValue(value).into();
                                highway_kind_name = match road_class.as_str() {
                                    "motorway" => Some("motorway"),
                                    "primary" => Some("primary"),
                                    "secondary" => Some("secondary"),
                                    "tertiary" => Some("tertiary"),
                                    "unclassified" => Some("unclassified"),
                                    "residential" => Some("residential"),
                                    "minor" => Some("residential"),
                                    "service" => Some("service"),
                                    "trunk" => Some("trunk"),
                                    _ => None,
                                };
                            } else if key == "brunnel" {
                                brunnel = true;
                            } else if key == "ramp" {
                                ramp = true;
                            }
                        }

                        if let Some(highway_kind_name) = highway_kind_name {
                            let mut bb = highway_kind_name.to_string();
                            if ramp {
                                bb = format!("{highway_kind_name}_link");
                            }
                            let highway_kind = HighwayKind::from_descr(bb.as_str()).unwrap();
                            let kk = 512.0f32 / 4096.0;
                            let geom_object = MapGeomObject {
                                id: -1,
                                kind: MapGeomObjectKind::Way(WayInfo {
                                    line_kind: LineKind::Highway { kind: highway_kind },
                                    layer: if brunnel { layer as i32 } else { 0 },
                                    layer_kind: LayerKind::None,
                                    name_en: None,
                                }),
                            };
                            for line in Self::get_all_lines(feature.geometry()?) {
                                let ls: LineString<f32> = line
                                    .coords_iter()
                                    .map(|coord| {
                                        coord! { x: coord.x as f32 * kk, y: coord.y as f32 * kk}
                                    })
                                    .collect();

                                res.push((geom_object.clone(), MapGeometry::Line(ls)));
                            }
                        }
                    }
                }
            } else if layer.name() == "water" {
                let kk = 512.0f32 / 4096.0;
                let geom_object = MapGeomObject {
                    id: -1,
                    kind: MapGeomObjectKind::Nature(NatureKind::Water),
                };
                for feature in layer.features() {
                    if let Some(geom_type) = feature.geom_type()
                        && geom_type == GeomType::POLYGON
                    {
                        match feature.geometry()? {
                            MvtGeometry::Polygon(poly) => {
                                let ls: Polygon<f32> = poly.map_coords(|coord| {
                                    coord! { x: coord.x as f32 * kk, y: coord.y as f32 * kk}
                                });
                                res.push((geom_object.clone(), MapGeometry::Poly(ls)));
                            }
                            MvtGeometry::MultiPolygon(polis) => {
                                for poly in polis {
                                    let ls: Polygon<f32> = poly.map_coords(|coord| {
                                        coord! { x: coord.x as f32 * kk, y: coord.y as f32 * kk}
                                    });

                                    res.push((geom_object.clone(), MapGeometry::Poly(ls)));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            } else if layer.name() == "landuse"
                || layer.name() == "landcover"
                || layer.name() == "park"
                || layer.name() == "grass"
                || layer.name() == "wood"
                // || layer.name() == "scrub"
                // || layer.name() == "forest"
                // || layer.name() == "vegetation"
            {
                // println!("layer: {:?}",layer.name());
                for feature in layer.features() {
                    // for property in feature.properties() {
                    //     let (key, value) = property?;
                    //     // println!("key: {:?}, value: {:?}", key, value);
                    //     if key == "class" || key != "class"  {
                    //         let class_type: String = LocalMvtValue(value).into();
                    //         // println!("ClassType: {}", class_type);
                    //         if layer.name() == "park"
                    //         || layer.name() == "grass"
                    //         || layer.name() == "wood"
                    //             || class_type == "wood"
                    //             || class_type == "park"
                    //             || class_type == "grass"
                    //             || class_type == "garden"
                    //             || class_type == "heath"
                    //             || class_type == "grassland"
                    //             || class_type == "nature_reserve"
                    //             || class_type == "geopark"
                    //             || class_type == "farmland"
                    //             || class_type == "national_park"
                    //         {
                    //
                    //         }
                    //     }
                    //     // println!("key: {:?}, value: {:?}",key, value);
                    // }

                    let kk = 512.0f32 / 4096.0;
                    let geom_object = MapGeomObject {
                        id: -1,
                        kind: MapGeomObjectKind::Nature(NatureKind::Forest),
                    };
                    if let Some(geom_type) = feature.geom_type()
                        && geom_type == GeomType::POLYGON
                    {
                        match feature.geometry()? {
                            MvtGeometry::Polygon(poly) => {
                                let ls: Polygon<f32> = poly.map_coords(|coord| {
                                    coord! { x: coord.x as f32 * kk, y: coord.y as f32 * kk}
                                });
                                res.push((geom_object.clone(), MapGeometry::Poly(ls)));
                            }
                            MvtGeometry::MultiPolygon(polis) => {
                                for poly in polis {
                                    let ls: Polygon<f32> = poly.map_coords(|coord| {
                                        coord! { x: coord.x as f32 * kk, y: coord.y as f32 * kk}
                                    });

                                    res.push((
                                        geom_object.clone(),
                                        MapGeometry::Poly(ls),
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            } else if layer.name() == "building" && tile_key.zoom_level >= MAX_ZOOM_LEVEL - 1 {
                let kk = 512.0f32 / 4096.0;
                let geom_object = MapGeomObject {
                    id: -1,
                    kind: MapGeomObjectKind::Building(3),
                };
                for feature in layer.features() {
                    if let Some(geom_type) = feature.geom_type()
                        && geom_type == GeomType::POLYGON
                    {
                        match feature.geometry()? {
                            MvtGeometry::Polygon(poly) => {
                                let ls: Polygon<f32> = poly.map_coords(|coord| {
                                    coord! { x: coord.x as f32 * kk, y: coord.y as f32 * kk}
                                });
                                res.push((geom_object.clone(), MapGeometry::Poly(ls)));
                            }
                            MvtGeometry::MultiPolygon(polis) => {
                                for poly in polis {
                                    let ls: Polygon<f32> = poly.map_coords(|coord| {
                                        coord! { x: coord.x as f32 * kk, y: coord.y as f32 * kk}
                                    });

                                    res.push((geom_object.clone(), MapGeometry::Poly(ls)));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        res.sort_by(|(a, _), (b, _)| a.cmp(b));

        let fixed = res
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
            MapGeometry::Line(line) => MapGeometry::Line(line.map_coords(|coord| {
                coord * factor
            })),
            MapGeometry::Poly(poly) => MapGeometry::Poly(poly.map_coords(|coord| {
                coord * factor
            })),
            MapGeometry::Coord(coord) => MapGeometry::Coord(
                *coord * factor,
            ),
        }
    }
}

struct LocalMvtValue<'a>(MvtValueRef<'a>);
impl From<LocalMvtValue<'_>> for i64 {
    fn from(value: LocalMvtValue<'_>) -> Self {
        match value.0 {
            MvtValueRef::SInt(value) => value,
            _ => panic!("Unexpected MvtValueRef"),
        }
    }
}

impl From<LocalMvtValue<'_>> for String {
    fn from(value: LocalMvtValue<'_>) -> Self {
        match value.0 {
            MvtValueRef::String(value) => value.to_string(),
            _ => panic!("Unexpected MvtValueRef"),
        }
    }
}
