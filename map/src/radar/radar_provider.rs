use crate::radar::radar_store::RadarStore;
use crate::radar::rain_contours::{process_tile, RainTileGeometry, RAIN_CATEGORIES};
use geo::BoundingRect;
use geo_types::{coord, LineString, Polygon, Rect};
use glam::DVec3;
use log::{error, info};
use lyon::geom::point;
use lyon::path::Path;
use rayon::prelude::*;
use renderer_common::geometry_data::{GeometryData, GeometryType, ShapeData, StyledRangeInfo};
use renderer_common::render_group::RenderGroup;
use renderer_common::render_modifier::SpatialData;
use renderer_common::render_style::RenderStyle;
use renderer_common::style_id::StyleId;
use renderer_common::{CanvasApi, RendererApi, MAP_SIZE, WEATHER_LAYER};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread::spawn;
use std::time::{Duration, Instant};

/// Rain data is ~1km per pixel at z7, higher zoom levels don't add details but cost API quota.
/// Higher map zoom levels reuse z7 geometry.
const MAX_RAIN_ZOOM: u32 = 7;
/// Upper bound of tiles requested for one view (e.g. a far-reaching pitched camera).
const MAX_TILES: usize = 48;
const SNAPSHOT_REFRESH: Duration = Duration::from_secs(10 * 60);
const SNAPSHOT_RETRY: Duration = Duration::from_secs(60);
const GEOMETRY_CACHE_SIZE: usize = 512;
const KEY_PREFIX: &str = "rain";

/// Fill per category in `RAIN_THRESHOLDS_DBZ` order: light, moderate, heavy.
/// Categories are nested and drawn on top of each other, so heavier rain gets darker.
const RAIN_COLORS: [[f32; 4]; RAIN_CATEGORIES] = [
    [0.55, 0.55, 0.55, 0.30],
    [0.35, 0.35, 0.35, 0.35],
    [0.15, 0.15, 0.15, 0.40],
];
const RAIN_STYLES: [&str; RAIN_CATEGORIES] = ["rain_light", "rain_moderate", "rain_heavy"];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct RadarTileId {
    z: u32,
    x: u32,
    y: u32,
}

impl RadarTileId {
    fn key(&self, snapshot: u64) -> String {
        format!("{KEY_PREFIX}/{snapshot}/{}/{}/{}", self.z, self.x, self.y)
    }

    fn world_size(&self) -> f64 {
        MAP_SIZE / (1u64 << self.z) as f64
    }

    fn spatial_data(&self) -> SpatialData {
        let size = self.world_size();
        let min_x = self.x as f64 * size;
        let min_y = self.y as f64 * size;
        SpatialData::transform(DVec3::new(min_x, min_y, 0.0)).bbox(Rect::new(
            coord! {x: min_x, y: min_y},
            coord! {x: min_x + size, y: min_y + size},
        ))
    }
}

/// Visible rain tiles for a given view.
#[derive(Clone, PartialEq, Eq)]
struct RadarView {
    tiles: Vec<RadarTileId>,
}

impl RadarView {
    fn new(polygon: &Polygon, map_zoom: i32) -> Option<Self> {
        let rect = polygon.bounding_rect()?;
        // Map zoom is in 512px tiles, rain tiles are 256px, so +1 keeps a similar density
        let mut z = (map_zoom + 1).clamp(0, MAX_RAIN_ZOOM as i32) as u32;
        loop {
            let tiles = Self::tiles(&rect, z);
            if tiles.len() <= MAX_TILES || z == 0 {
                return Some(Self { tiles });
            }
            z -= 1;
        }
    }

    fn tiles(rect: &Rect, z: u32) -> Vec<RadarTileId> {
        let n = 1i64 << z;
        let to_tile = |v: f64| (v / MAP_SIZE * n as f64).floor() as i64;
        let (min_x, max_x) = (to_tile(rect.min().x), to_tile(rect.max().x));
        let min_y = to_tile(rect.min().y).clamp(0, n - 1);
        let max_y = to_tile(rect.max().y).clamp(0, n - 1);
        // the view may cross the antimeridian, wrap x but don't produce duplicates
        let max_x = max_x.min(min_x + n - 1);

        let mut tiles = Vec::new();
        for x in min_x..=max_x {
            for y in min_y..=max_y {
                tiles.push(RadarTileId { z, x: x.rem_euclid(n) as u32, y: y as u32 });
            }
        }
        tiles
    }
}

struct RainTile(Arc<RainTileGeometry>);

impl RainTile {
    fn path(polygon: &Polygon<f32>) -> Path {
        let mut builder = Path::builder();
        let mut add_ring = |ring: &LineString<f32>| {
            let mut coords = ring.coords();
            if let Some(first) = coords.next() {
                builder.begin(point(first.x, first.y));
                // the ring is closed, skip the duplicated last point
                coords.take(ring.0.len().saturating_sub(2)).for_each(|c| {
                    builder.line_to(point(c.x, c.y));
                });
                builder.end(true);
            }
        };
        // holes are cut out by the even-odd fill rule
        add_ring(polygon.exterior());
        polygon.interiors().iter().for_each(add_ring);
        builder.build()
    }
}

impl<T: CanvasApi> RenderGroup<T> for RainTile {
    fn content(self: Box<Self>, canvas: &mut T) {
        canvas.set_feature_layer_tag(Some(WEATHER_LAYER.to_string()));
        for (category, polygons) in self.0.categories.iter().enumerate() {
            for polygon in polygons {
                canvas.geometry_data(GeometryData::Shape(ShapeData {
                    path: Self::path(polygon),
                    geometry_type: GeometryType::Polygon,
                    style_id: StyleId::new(RAIN_STYLES[category]),
                    // light -> moderate -> heavy drawing order
                    index_layer_level: category as i16,
                    styled_range_info: StyledRangeInfo::new(1, true),
                }));
            }
        }
    }
}

/// Fetches the latest rainbow.ai precipitation (radar) snapshot for the visible area,
/// converts it to light/moderate/heavy rain polygons and submits them to the renderer.
/// Runs on its own thread.
pub struct RadarProvider {
    sender: Option<Sender<RadarView>>,
    last_view: Option<RadarView>,
}

impl RadarProvider {
    pub fn new<RAPI: RendererApi + 'static>(renderer_api: Arc<RAPI>) -> Self {
        let disabled = Self { sender: None, last_view: None };
        let Some(api_key) = option_env!("RAINBOW_API_KEY") else {
            error!("RAINBOW_API_KEY is not set, rain layer is disabled");
            return disabled;
        };
        let store = match RadarStore::new(api_key) {
            Ok(store) => store,
            Err(e) => {
                error!("Can't create rain store, rain layer is disabled: {e}");
                return disabled;
            }
        };

        for (style, color) in RAIN_STYLES.iter().zip(RAIN_COLORS) {
            renderer_api.update_style(StyleId::new(*style), move |s| *s = RenderStyle::fill(color));
        }

        let (sender, receiver) = channel::<RadarView>();
        spawn(move || {
            let mut worker = RadarWorker {
                store,
                renderer_api,
                snapshot: None,
                next_snapshot_check: Instant::now(),
                view: None,
                shown: HashSet::new(),
                cache: GeometryCache::default(),
            };
            loop {
                let timeout = worker.next_snapshot_check.saturating_duration_since(Instant::now());
                let mut view_changed = false;
                match receiver.recv_timeout(timeout) {
                    Ok(view) => {
                        // only the most recent view matters
                        worker.view = Some(receiver.try_iter().last().unwrap_or(view));
                        view_changed = true;
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                let snapshot_changed = worker.refresh_snapshot_if_needed();
                if view_changed || snapshot_changed {
                    worker.sync_tiles();
                }
            }
        });

        Self { sender: Some(sender), last_view: None }
    }

    /// Cheap to call every frame, the worker is notified only when the visible tile set changes.
    pub fn update_view(&mut self, polygon: &Polygon, map_zoom: i32) {
        let Some(sender) = &self.sender else { return };
        let Some(view) = RadarView::new(polygon, map_zoom) else { return };
        if self.last_view.as_ref() != Some(&view) {
            self.last_view = Some(view.clone());
            let _ = sender.send(view);
        }
    }
}

/// Processed geometry of the current snapshot, so revisited areas don't need fetching/processing.
/// Dry tiles are cached as empty geometry.
#[derive(Default)]
struct GeometryCache {
    tiles: HashMap<RadarTileId, Arc<RainTileGeometry>>,
    order: VecDeque<RadarTileId>,
}

impl GeometryCache {
    fn get(&self, tile: &RadarTileId) -> Option<Arc<RainTileGeometry>> {
        self.tiles.get(tile).cloned()
    }

    fn insert(&mut self, tile: RadarTileId, geometry: Arc<RainTileGeometry>) {
        if self.tiles.insert(tile, geometry).is_none() {
            self.order.push_back(tile);
        }
        while self.order.len() > GEOMETRY_CACHE_SIZE {
            if let Some(oldest) = self.order.pop_front() {
                self.tiles.remove(&oldest);
            }
        }
    }

    fn clear(&mut self) {
        self.tiles.clear();
        self.order.clear();
    }
}

struct RadarWorker<RAPI: RendererApi> {
    store: RadarStore,
    renderer_api: Arc<RAPI>,
    snapshot: Option<u64>,
    next_snapshot_check: Instant,
    view: Option<RadarView>,
    // render group keys currently submitted to the renderer
    shown: HashSet<String>,
    cache: GeometryCache,
}

impl<RAPI: RendererApi> RadarWorker<RAPI> {
    /// Returns true if a new snapshot is available.
    fn refresh_snapshot_if_needed(&mut self) -> bool {
        if Instant::now() < self.next_snapshot_check {
            return false;
        }
        match self.store.latest_snapshot() {
            Some(snapshot) => {
                self.next_snapshot_check = Instant::now() + SNAPSHOT_REFRESH;
                if self.snapshot != Some(snapshot) {
                    info!("New rain snapshot: {snapshot}");
                    self.snapshot = Some(snapshot);
                    self.cache.clear();
                    return true;
                }
                false
            }
            None => {
                self.next_snapshot_check = Instant::now() + SNAPSHOT_RETRY;
                false
            }
        }
    }

    fn sync_tiles(&mut self) {
        let (Some(snapshot), Some(view)) = (self.snapshot, &self.view) else { return };
        let wanted: Vec<(String, RadarTileId)> = view
            .tiles
            .iter()
            .map(|tile| (tile.key(snapshot), *tile))
            .collect();

        let mut next_shown = HashSet::new();
        let mut to_send = Vec::new();
        let mut to_load = Vec::new();
        for (key, tile) in wanted {
            if self.shown.contains(&key) {
                next_shown.insert(key);
            } else if let Some(geometry) = self.cache.get(&tile) {
                to_send.push((key, tile, geometry));
            } else {
                to_load.push((key, tile));
            }
        }
        let cache_hits = to_send.len();

        let loaded: Vec<(String, RadarTileId, Arc<RainTileGeometry>, Duration, Duration)> = to_load
            .into_par_iter()
            .filter_map(|(key, tile)| {
                let t0 = Instant::now();
                let png = self.store.load_tile(snapshot, tile.z, tile.x, tile.y)?;
                let fetch_time = t0.elapsed();
                let t1 = Instant::now();
                let geometry = process_tile(&png, tile.z, tile.world_size() as f32)
                    .inspect_err(|e| error!("Rain tile {tile:?} processing failed: {e}"))
                    .ok()?;
                Some((key, tile, Arc::new(geometry), fetch_time, t1.elapsed()))
            })
            .collect();

        if !loaded.is_empty() {
            let stats = |times: Vec<Duration>| {
                let min = times.iter().min().copied().unwrap_or_default();
                let max = times.iter().max().copied().unwrap_or_default();
                let avg = times.iter().sum::<Duration>() / times.len() as u32;
                format!("min {min:.1?}, avg {avg:.1?}, max {max:.1?}")
            };
            info!(
                "Rain tiles: {} loaded, {} from cache; fetch: {}; processing: {}",
                loaded.len(),
                cache_hits,
                stats(loaded.iter().map(|l| l.3).collect()),
                stats(loaded.iter().map(|l| l.4).collect()),
            );
        }

        for (key, tile, geometry, ..) in loaded {
            self.cache.insert(tile, geometry.clone());
            to_send.push((key, tile, geometry));
        }

        for (key, tile, geometry) in to_send {
            // dry tiles are not sent at all, but count as shown to avoid re-checking them
            if !geometry.is_empty() {
                self.renderer_api.add_render_group(key.clone(), tile.spatial_data(), RainTile(geometry));
            }
            next_shown.insert(key);
        }

        // remove outdated tiles only after the new ones are submitted to avoid blinking
        let to_remove: HashSet<String> = self.shown.difference(&next_shown).cloned().collect();
        if !to_remove.is_empty() {
            self.renderer_api.clear_render_groups(to_remove);
        }
        self.shown = next_shown;
    }
}
