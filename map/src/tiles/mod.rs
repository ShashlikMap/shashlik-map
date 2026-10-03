use derivative::Derivative;
use osm::map::{LayerKind, LineKind, MapGeomObject};
use std::cmp::Ordering;

pub mod default_tiles_provider;
mod grid_divider;
pub mod mvt;
pub mod shashlik;
pub mod shashlik_v1;
pub mod tile_data;
mod tile_parser;
pub mod tiles_provider;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShashlikMapGeomObject {
    pub id: i64,
    pub kind: ShashlikMapGeomObjectKind,
}

#[derive(Debug, Clone, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub enum ShashlikMapGeomObjectKind {
    Nature(ShashlikNatureKind),
    Building(u16),
    Way(ShashlikWayInfo),
    AdminLine,
    Poi(ShashlikMapPointInfo),
}

// FIXME PartialOrd and Ord are not aligned, so fat it's causing any issues.
// After moving LineKind here, it's probably better to just impl custom PartialOrd/Ord for LineKind only
// and use deriving for ShashlikWayInfo
#[derive(Derivative, Debug, Clone)]
#[derivative(PartialEq, PartialOrd, Hash, Eq)]
pub struct ShashlikWayInfo {
    pub line_kind: LineKind,
    pub layer: i32,
    pub layer_kind: LayerKind,
    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    #[derivative(PartialOrd = "ignore")]
    pub name_en: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShashlikMapPointInfo {
    pub text: String,
    pub kind: ShashlikMapPointObjectKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ShashlikNatureKind {
    Ground,
    Park,
    Forest,
    Water,
}

#[derive(Debug, Clone, Copy, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub enum ShashlikMapPointObjectKind {
    PopArea(ShashlikPopAreaInfo),
    TrafficLight,
    Crossing,
    Toilet,
    Parking,
    EVCharging,
    TrainStation(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq)]
pub struct ShashlikPopAreaInfo {
    pub level: i32,
    pub population: u32,
}

impl Ord for ShashlikMapGeomObject {
    fn cmp(&self, other: &Self) -> Ordering {
        self.kind.cmp(&other.kind)
    }
}

impl PartialOrd for ShashlikMapGeomObject {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialOrd for ShashlikPopAreaInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ShashlikPopAreaInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.level.cmp(&other.level) {
            Ordering::Equal => self.population.cmp(&other.population),
            v => v,
        }
    }
}

impl Ord for ShashlikMapPointInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.kind.cmp(&other.kind)
    }
}

impl PartialOrd for ShashlikMapPointInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ShashlikWayInfo {
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


