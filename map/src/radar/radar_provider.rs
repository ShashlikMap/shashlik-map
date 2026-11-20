use crate::radar::radar_store::RadarStore;
use geo::BoundingRect;
use geo_types::{coord, Polygon, Rect};
use glam::DVec3;
use log::{error, info};
use rayon::prelude::*;
use renderer_common::geometry_data::{GeometryData, RasterData};
use renderer_common::render_group::RenderGroup;
use renderer_common::render_modifier::SpatialData;
use renderer_common::{CanvasApi, RendererApi, MAP_SIZE};
use std::collections::HashSet;
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread::spawn;
use std::time::{Duration, Instant};

/// rainbow.ai radar tiles exist only up to this zoom; higher map zooms stretch these tiles.
const MAX_RADAR_ZOOM: u32 = 7;
/// Upper bound of tiles requested for one view (e.g. a far-reaching pitched camera).
const MAX_TILES: usize = 48;
const SNAPSHOT_REFRESH: Duration = Duration::from_secs(10 * 60);
const SNAPSHOT_RETRY: Duration = Duration::from_secs(60);
const KEY_PREFIX: &str = "radar";

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

/// Visible radar tiles for a given view.
#[derive(Clone, PartialEq, Eq)]
struct RadarView {
    tiles: Vec<RadarTileId>,
}

impl RadarView {
    fn new(polygon: &Polygon, map_zoom: i32) -> Option<Self> {
        let rect = polygon.bounding_rect()?;
        // Map zoom is in 512px tiles, radar tiles are 256px, so +1 keeps a similar texel density
        let mut z = (map_zoom + 1).clamp(0, MAX_RADAR_ZOOM as i32) as u32;
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

struct RadarTile {
    encoded: Vec<u8>,
    world_size: f32,
}

impl<T: CanvasApi> RenderGroup<T> for RadarTile {
    fn content(self: Box<Self>, canvas: &mut T) {
        canvas.geometry_data(GeometryData::Raster(RasterData {
            encoded: self.encoded,
            world_size: self.world_size,
        }));
    }
}

/// Fetches the latest rainbow.ai precipitation radar snapshot for the visible area and
/// submits it to the renderer as raster render groups. Runs on its own thread.
pub struct RadarProvider {
    sender: Option<Sender<RadarView>>,
    last_view: Option<RadarView>,
}

impl RadarProvider {
    pub fn new<RAPI: RendererApi + 'static>(renderer_api: Arc<RAPI>) -> Self {
        let Some(api_key) = option_env!("RAINBOW_API_KEY") else {
            error!("RAINBOW_API_KEY is not set, rain radar is disabled");
            return Self { sender: None, last_view: None };
        };
        let store = match RadarStore::new(api_key) {
            Ok(store) => store,
            Err(e) => {
                error!("Can't create radar store, rain radar is disabled: {e}");
                return Self { sender: None, last_view: None };
            }
        };

        let (sender, receiver) = channel::<RadarView>();
        spawn(move || {
            let mut worker = RadarWorker {
                store,
                renderer_api,
                snapshot: None,
                next_snapshot_check: Instant::now(),
                view: None,
                shown: HashSet::new(),
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

struct RadarWorker<RAPI: RendererApi> {
    store: RadarStore,
    renderer_api: Arc<RAPI>,
    snapshot: Option<u64>,
    next_snapshot_check: Instant,
    view: Option<RadarView>,
    // render group keys currently submitted to the renderer
    shown: HashSet<String>,
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
                    info!("New radar snapshot: {snapshot}");
                    self.snapshot = Some(snapshot);
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

        let to_load: Vec<_> = wanted
            .iter()
            .filter(|(key, _)| !self.shown.contains(key))
            .collect();

        let loaded: Vec<(String, RadarTileId, Vec<u8>)> = to_load
            .par_iter()
            .filter_map(|(key, tile)| {
                self.store
                    .load_tile(snapshot, tile.z, tile.x, tile.y)
                    .map(|data| (key.clone(), *tile, data))
            })
            .collect();

        let mut next_shown: HashSet<String> = wanted
            .iter()
            .filter(|(key, _)| self.shown.contains(key))
            .map(|(key, _)| key.clone())
            .collect();

        for (key, tile, encoded) in loaded {
            self.renderer_api.add_render_group(
                key.clone(),
                tile.spatial_data(),
                RadarTile { encoded, world_size: tile.world_size() as f32 },
            );
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
