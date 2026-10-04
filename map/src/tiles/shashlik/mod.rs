use crate::MAX_ZOOM_LEVEL;
use crate::tiles::tiles_provider::{MercatorConverter, MercatorProvider, TilesProviderStore};
use crate::tiles::{ShashlikMapGeomObject, ShashlikMapGeomObjectKind, ShashlikMapPointInfo, ShashlikMapPointObjectKind, ShashlikNatureKind, ShashlikPopAreaInfo, ShashlikWayInfo};
use geo::{BoundingRect, Intersects, MapCoordsInPlace, Scale};
use geo_types::{Coord, Polygon, Rect};
use glam::DVec3;
use googleprojection::Mercator;
use osm::map::{MapGeomObject, MapGeomObjectKind, MapGeometry, MapPointInfo, MapPointObjectKind, NatureKind, PopAreaInfo, WayInfo};
use osm::source::TileSource;
use osm::tiles::{TILES_COUNT, TILE_OVERLAP_PERCENT, TILE_SIZE, TileKey, TileStore, calc_tile_ranges};
use std::collections::HashSet;

impl<S: TileSource> MercatorProvider for TileStore<S> {
    fn mercator(&self) -> Mercator {
        Mercator::with_size(TILE_SIZE)
    }
}


/// TileStore uses hardcoded zoom 22, so the caller's zoom has to be ignored
impl<S: TileSource> MercatorConverter for TileStore<S> {
    fn lon_lat_to_world(&self, lon_lat: &Coord<f64>, _zoom_level: i32) -> Coord<f64> {
        let lon_lat: (f64, f64) = (*lon_lat).into();

        self.mercator()
            .from_ll_to_subpixel(&lon_lat, 22)
            .unwrap()
            .into()
    }

    fn world_to_lon_lat(&self, xy: &Coord<f64>, _zoom_level: i32) -> Coord<f64> {
        let xy: (f64, f64) = (*xy).into();
        self.mercator()
            .from_pixel_to_ll(&xy, 22)
            .unwrap()
            .into()
    }
}

impl <S:TileSource> TilesProviderStore for TileStore<S> {
    fn convert_zoom(&self, zoom_level: i32) -> i32 {
        MAX_ZOOM_LEVEL - zoom_level
    }

    fn tile_ranges(&self, mut area: Polygon<f64>, zoom_level: i32) -> HashSet<TileKey> {
        let zoom_level = self.convert_zoom(zoom_level);

        area.map_coords_in_place(|coord| {
            self.world_to_lon_lat(&coord, zoom_level)
        });

        // this will be compared for intersection later, it should have a correct winding
        let area_lon_lat = area.exterior().bounding_rect().unwrap();

        let ranges = calc_tile_ranges(TILES_COUNT, zoom_level, &area_lon_lat);
        let mut res = HashSet::new();
        for tx in ranges.min_x..=ranges.max_x {
            for ty in ranges.min_y..=ranges.max_y {
                let tile_key = TileKey {
                    tile_x: tx as i32,
                    tile_y: ty as i32,
                    zoom_level,
                };

                // FIXME Maybe move "calc_tile_boundary" to tile generator? since we need to calculate all the time and twice(+ before loading)
                let tile_rect = tile_key.calc_tile_boundary(1.0);
                if area.intersects(&tile_rect) {
                    res.insert(tile_key);
                }
            }
        }
        res
    }

    // fyi, TilesV0 won't support infinite scroll + it'll be removed anyway soon
    fn tile_position_bbox(&self, tile_key: &TileKey, bbox_scale: f64) -> (DVec3, Rect) {
        let tile_rect = tile_key.calc_tile_boundary(TILE_OVERLAP_PERCENT);

        let tile_rect_origin = self.lon_lat_to_world(&tile_rect.min(), MAX_ZOOM_LEVEL);
        let tile_position = [tile_rect_origin.x, tile_rect_origin.y, 0.0].into();

        let tile_rect_original = tile_key.calc_tile_boundary(1.00);
        let tile_rect_original_min = self.lon_lat_to_world(&tile_rect_original.min(), MAX_ZOOM_LEVEL);
        let tile_rect_original_max = self.lon_lat_to_world(&tile_rect_original.max(), MAX_ZOOM_LEVEL);
        let bbox = Rect::new(tile_rect_original_min, tile_rect_original_max).scale(bbox_scale);
        (tile_position, bbox)
    }

    fn load(&self, tile_key: &TileKey) -> Vec<(ShashlikMapGeomObject, MapGeometry<f32>)> {
        self.load_geometries(tile_key).into_iter().map(|(geom_obj, geom)| {
            (geom_obj.into(), geom)
        }).collect()
    }
}

impl From<MapGeomObject> for ShashlikMapGeomObject {
    fn from(value: MapGeomObject) -> Self {
        ShashlikMapGeomObject {
            id: value.id,
            kind: value.kind.into(),
        }
    }
}

impl From<MapGeomObjectKind> for ShashlikMapGeomObjectKind {
    fn from(value: MapGeomObjectKind) -> Self {
        match value {
            MapGeomObjectKind::Nature(data) => ShashlikMapGeomObjectKind::Nature(data.into()),
            MapGeomObjectKind::Building(data) => ShashlikMapGeomObjectKind::Building(data),
            MapGeomObjectKind::Way(data) => ShashlikMapGeomObjectKind::Way(data.into()),
            MapGeomObjectKind::AdminLine => ShashlikMapGeomObjectKind::AdminLine,
            MapGeomObjectKind::Poi(data) => ShashlikMapGeomObjectKind::Poi(data.into())
        }
    }
}

impl From<WayInfo> for ShashlikWayInfo {
    fn from(value: WayInfo) -> Self {
        match value {
            WayInfo { .. } => ShashlikWayInfo {
                line_kind: value.line_kind,
                layer: value.layer,
                layer_kind: value.layer_kind,
                name_en: value.name_en,
                linear_refs: vec![]
            }
        }
    }
}

impl From<NatureKind> for ShashlikNatureKind {
    fn from(value: NatureKind) -> Self {
        match value {
            NatureKind::Ground => ShashlikNatureKind::Ground,
            NatureKind::Park => ShashlikNatureKind::Park,
            NatureKind::Forest => ShashlikNatureKind::Forest,
            NatureKind::Water => ShashlikNatureKind::Water
        }
    }
}

impl From<MapPointInfo> for ShashlikMapPointInfo {
    fn from(value: MapPointInfo) -> Self {
        match value {
            MapPointInfo { .. } => ShashlikMapPointInfo {
                text: value.text,
                kind: value.kind.into()
            }
        }
    }
}

impl From<PopAreaInfo> for ShashlikPopAreaInfo {
    fn from(value: PopAreaInfo) -> Self {
        ShashlikPopAreaInfo {
            level: value.level,
            population: value.population,
        }
    }
}

impl From<MapPointObjectKind> for ShashlikMapPointObjectKind {
    fn from(value: MapPointObjectKind) -> Self {
        match value {
            MapPointObjectKind::PopArea(data) => ShashlikMapPointObjectKind::PopArea(data.into()),
            MapPointObjectKind::TrafficLight => ShashlikMapPointObjectKind::TrafficLight,
            MapPointObjectKind::Toilet => ShashlikMapPointObjectKind::Toilet,
            MapPointObjectKind::Parking => ShashlikMapPointObjectKind::Parking,
            MapPointObjectKind::EVCharging => ShashlikMapPointObjectKind::EVCharging,
            MapPointObjectKind::TrainStation(data) => ShashlikMapPointObjectKind::TrainStation(data)
        }
    }
}