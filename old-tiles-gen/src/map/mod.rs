//! Map types

use derivative::Derivative;
use geo::{coord, point, BoundingRect, Coord, CoordNum, LineString, Point, Polygon, Rect};
use rstar::{Envelope, PointDistance, RTreeObject, AABB};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub const DBS_FOLDER: &str = "dbs";
// 18 is quite far, no need more than that
pub const ZOOM_LEVELS: u32 = 18;

pub fn get_world_boundary() -> Rect {
    Rect::new(
        Coord {
            x: -180.0,
            y: 89.0,
        },
        Coord {
            x: 180.0,
            y: -75.0,
        },
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MapGeomObject {
    pub id: i64,
    pub kind: MapGeomObjectKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MapPointInfo {
    pub text: String,
    pub kind: MapPointObjectKind,
}

#[derive(Eq, PartialEq, Clone, Debug, Hash, Serialize, Deserialize)]
pub enum MapGeometry<T: CoordNum = f64> {
    Line(LineString<T>),
    Poly(Polygon<T>),
    Coord(Coord<T>)
}

impl<T> BoundingRect<T> for MapGeometry<T>
where
    T: CoordNum,
{
    type Output = Option<Rect<T>>;

    fn bounding_rect(&self) -> Self::Output {
        match self {
            MapGeometry::Line(line) => line.bounding_rect(),
            MapGeometry::Poly(poly) => poly.bounding_rect(),
            MapGeometry::Coord(coord) => Some(coord.bounding_rect()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct MapGeometryCollection(pub Vec<(MapGeomObject, MapGeometry)>);

// TODO Remove after fully implemented in renderer
impl<T: std::fmt::Debug + CoordNum> MapGeometry<T> {
    pub fn line_string(&self) -> &LineString<T> {
        match self {
            MapGeometry::Line(c) => Some(c),
            _ => None,
        }
        .unwrap()
    }

    pub fn polygon(&self) -> &Polygon<T> {
        match self {
            MapGeometry::Poly(c) => Some(c),
            _ => None,
        }
        .unwrap()
    }

    pub fn coord(&self) -> &Coord<T> {
        match self {
            MapGeometry::Coord(c) => Some(c),
            _ => None,
        }
            .unwrap()
    }
}

impl RTreeObject for MapGeometry {
    type Envelope = AABB<Point>;

    fn envelope(&self) -> Self::Envelope {
        match self {
            MapGeometry::Line(line) => line.envelope(),
            MapGeometry::Poly(poly) => poly.envelope(),
            MapGeometry::Coord(coord) => point!(x: coord.x, y: coord.y).envelope()
        }
    }
}

impl PointDistance for MapGeometry {
    fn distance_2(&self, point: &<Self::Envelope as Envelope>::Point) -> <<Self::Envelope as Envelope>::Point as rstar::Point>::Scalar {
        match self {
            MapGeometry::Line(line) => line.distance_2(point),
            MapGeometry::Poly(poly) => poly.exterior().distance_2(point),
            MapGeometry::Coord(coord) => {coord.distance_2(&coord! {x: point.x(), y: point.y()}) }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub enum MapPointObjectKind {
    PopArea(PopAreaInfo),
    TrafficLight,
    Toilet,
    Parking,
    Text
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub struct PopAreaInfo {
    pub level: i32,
    pub population: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub enum MapGeomObjectKind {
    Nature(NatureKind),
    Building,
    Way(WayInfo),
    Route,
    AdminLine,
    Poi(MapPointInfo)
}

impl MapGeomObjectKind {
    pub fn from_tag(k: &str, v: &str, way_info: Option<WayInfo>) -> MapGeomObjectKind {
        match k {
            "highway" => {
                if v == "traffic_signals" {
                    MapGeomObjectKind::Poi(MapPointInfo {
                        text: "".to_string(),
                        kind: MapPointObjectKind::TrafficLight,
                    })
                } else {
                    MapGeomObjectKind::Way(way_info.unwrap())
                }
            }
            "amenity" => {
                if v == "toilets" {
                    MapGeomObjectKind::Poi(MapPointInfo {
                        text: "".to_string(),
                        kind: MapPointObjectKind::Toilet,
                    })
                } else if v == "parking" {
                    MapGeomObjectKind::Poi(MapPointInfo {
                        text: "".to_string(),
                        kind: MapPointObjectKind::Parking,
                    })
                } else {
                    panic!("Unknown key/value: {},{}", k, v)
                }
            }
            "railway" => MapGeomObjectKind::Way(way_info.unwrap()),
            "water" => MapGeomObjectKind::Nature(NatureKind::Water),
            "leisure" => MapGeomObjectKind::Nature(NatureKind::Park),
            "building" => MapGeomObjectKind::Building,
            "natural" | "landuse" => {
                if v == "water" || v == "bay" {
                    MapGeomObjectKind::Nature(NatureKind::Water)
                } else {
                    MapGeomObjectKind::Nature(NatureKind::Forest)
                }
            }
            &_ => { panic!("Unknown key/value: {},{}", k, v) }
        }
    }
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NatureKind {
    Ground,
    Park,
    Forest,
    Water,
}

#[derive(Derivative, Debug, Clone, Serialize, Deserialize)]
#[derivative(PartialEq, PartialOrd, Hash, Eq)]
pub struct WayInfo {
    pub line_kind: LineKind,
    pub layer: i32,
    pub layer_kind: LayerKind,
    #[derivative(PartialEq="ignore")]
    #[derivative(Hash="ignore")]
    #[derivative(PartialOrd="ignore")]
    pub name_en: Option<String>
}

impl Ord for MapGeomObject {
    fn cmp(&self, other: &Self) -> Ordering {
        self.kind.cmp(&other.kind)
    }
}

impl PartialOrd for MapGeomObject {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MapPointInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.kind.cmp(&other.kind)
    }
}

impl PartialOrd for MapPointInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WayInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        // Bridge has priority over tunnels even if the tunnel has the higher layer!
        // Example can be found here: https://www.openstreetmap.org/way/80581130
        // Tunnel has layer 3, but it's below than the bridge with layer 2!

        // sort by OSM layer_kind layer first, then by layer itself and only then by internal layer values
        match self.layer_kind.cmp(&other.layer_kind) {
            Ordering::Equal => match self.layer.cmp(&other.layer) {
                Ordering::Equal => self.line_kind.get_layer().cmp(&other.line_kind.get_layer()),
                v => v,
            },
            v => v,
        }
    }
}

impl core::cmp::PartialOrd for PopAreaInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PopAreaInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.level.cmp(&other.level) {
            Ordering::Equal => self.population.cmp(&other.population),
            v => v,
        }
    }
}

/// https://wiki.openstreetmap.org/wiki/Key:highway
#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd,
)]
pub enum HighwayKind {
    Motorway,
    Trunk,
    #[default]
    Primary,
    Secondary,
    Tertiary,
    Unclassified,
    Residential,
    MotorwayLink,
    TrunkLink,
    PrimaryLink,
    SecondaryLink,
    TertiaryLink,
    Service,
    Footway
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum LayerKind {
    Tunnel,
    None,
    Bridge,
}

impl HighwayKind {
    // links usually should not have caps
    pub fn is_it_link(self) -> bool {
        matches!(self, HighwayKind::MotorwayLink
            | HighwayKind::TrunkLink
            | HighwayKind::PrimaryLink
            | HighwayKind::SecondaryLink
            | HighwayKind::TertiaryLink)
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Motorway => "motorway",
            Self::Trunk => "trunk",
            Self::Primary => "primary",
            Self::Secondary => "secondary",
            Self::Tertiary => "tertiary",
            Self::Unclassified => "unclassified",
            Self::Residential => "residential",
            Self::MotorwayLink => "motorway_link",
            Self::TrunkLink => "trunk_link",
            Self::PrimaryLink => "primary_link",
            Self::SecondaryLink => "secondary_link",
            Self::TertiaryLink => "tertiary_link",
            Self::Service => "service",
            Self::Footway => "footway",
        }
    }

    pub fn from_descr(val: &str) -> Option<Self> {
        match val {
            "motorway" => Some(Self::Motorway),
            "trunk" => Some(Self::Trunk),
            "primary" => Some(Self::Primary),
            "secondary" => Some(Self::Secondary),
            "tertiary" => Some(Self::Tertiary),
            "unclassified" => Some(Self::Unclassified),
            "residential" => Some(Self::Residential),
            "motorway_link" => Some(Self::MotorwayLink),
            "trunk_link" => Some(Self::TrunkLink),
            "primary_link" => Some(Self::PrimaryLink),
            "secondary_link" => Some(Self::SecondaryLink),
            "tertiary_link" => Some(Self::TertiaryLink),
            "service" => Some(Self::Service),
            "footway" => Some(Self::Footway),
            _ => None,
        }
    }

    pub fn get_layer(&self) -> u16 {
        // order based on OSM observation, might not be correct
        // there should be some proper order
        match self {
            Self::Motorway => 16,
            Self::Trunk => 15,
            Self::Primary => 14,
            Self::Secondary => 13,
            Self::Tertiary => 12,
            Self::MotorwayLink => 11,
            Self::TrunkLink => 10,
            Self::PrimaryLink => 9,
            Self::SecondaryLink => 8,
            Self::TertiaryLink => 7,
            Self::Residential => 6,
            Self::Service => 5,
            Self::Unclassified => 4,
            Self::Footway => 3,
        }
    }
}

#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd,
)]
pub enum RailwayKind {
    #[default]
    Rail,
}

impl RailwayKind {
    pub fn get_layer(&self) -> u16 {
        match self {
            // should be above all roads withing a layer
            RailwayKind::Rail => 17,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum LineKind {
    Highway { kind: HighwayKind },
    Railway { kind: RailwayKind },
}

impl LineKind {
    pub fn get_layer(&self) -> u16 {
        match self {
            Self::Highway { kind } => kind.get_layer(),
            Self::Railway { kind } => kind.get_layer(),
        }
    }

    pub fn is_it_link(&self) -> bool {
        match self {
            Self::Highway { kind } => kind.is_it_link(),
            Self::Railway { .. } => true,
        }
    }
}

impl std::default::Default for LineKind {
    fn default() -> Self {
        Self::Highway {
            kind: HighwayKind::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinePath {
    pub kind: LineKind,
    pub line_path: LineString,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum PolygonKind {
    Water,
    Park,
    Forest,
    Building,
}

impl PolygonKind {
    pub fn get_layer(&self) -> u16 {
        match self {
            Self::Park => 0,
            Self::Forest => 1,
            Self::Water => 3,
            Self::Building => 4,
        }
    }
}
