# Arbitrary Track Rendering

## Context

`TripHistoryPoC` needs to display pre-recorded GPS tracks on the map — on trip detail screens and
optionally as a live trail during recording. These are raw or map-matched coordinate sequences
fetched from a server, not Valhalla-routed paths.

The current `ShashlikMapApi` has no method to draw an arbitrary polyline. `calculate_route_to_lat_lon`
is unsuitable because it produces a freshly-routed path, not the recorded one.

---

## Requirement

Expose two methods on `ShashlikMapApi` (via uniffi):

```kotlin
fun drawTrack(points: List<LatLon>)
fun clearTrack()
```

**Behaviour:**
- `drawTrack` replaces any previously drawn track on the next render call
- Rendered as a continuous polyline, visually distinct from Valhalla routes
- `clearTrack` removes it
- No camera movement — the caller manages the viewport via `setLatLonBearing` or gestures
- Safe to call at GPS update frequency (1 Hz) for live trail use cases

---

## Implementation Plan

### Files to change

| File | Change |
|---|---|
| `map/src/track_group.rs` | New — `TrackGroup` render group |
| `map/src/lib.rs` | Add `draw_track` / `clear_track` to `ShashlikMap` |
| `ffi-run/src/lib.rs` | Add `LatLon` record + two uniffi methods to `ShashlikMapApi` |

No changes needed outside the `map` and `ffi-run` crates.

---

### Step 1 — `TrackGroup` (`map/src/track_group.rs`)

A new struct implementing `RenderGroup`, structurally similar to `RouteGroup` for car/motorbike
(the Lyon polyline path, no pedestrian dotted variant needed).

Key details:
- Points are stored as `Vec<Point>` in world space (already converted from lon/lat by the caller)
- The path is built relative to `first_point()` — same floating-point precision pattern used in
  `RouteGroup` — to avoid large absolute world coordinates on the GPU
- `SpatialData::transform(first_point())` carries the absolute world offset
- Style ID: `"track"` (see Step 2 for registration)
- Feature layer tag: `"kml_layer"` — already a non-indirect default feature layer that renders
  after tile geometry, giving correct z-ordering for free with no new layer registration needed

```rust
pub struct TrackGroup {
    points: Vec<Point>,
}

impl TrackGroup {
    pub fn new(points: Vec<Point>) -> Self { ... }
    pub fn first_point(&self) -> DVec3 { ... }
}

impl<T: CanvasApi> RenderGroup<T> for TrackGroup {
    fn content(&mut self, canvas: &mut T) {
        canvas.set_feature_layer_tag(Some("kml_layer".to_string()));
        // build Lyon path relative to first_point, submit as GeometryData::Shape polyline
    }
}
```

---

### Step 2 — Style registration (`map/src/lib.rs`, `ShashlikMap::new`)

`StyleStore::get_index` silently inserts a default style for unknown IDs, so the track renders
immediately even without explicit registration. To give it a distinct appearance, register the
style once in `ShashlikMap::new` alongside the existing puck setup:

```rust
renderer.api().update_style(StyleId::new("track"), |s| {
    *s = RenderStyle::fill([0.2, 0.5, 1.0, 0.85]); // blue, visually distinct from route orange
});
```

No external `osm` crate change required. The feature is fully self-contained.

---

### Step 3 — `draw_track` / `clear_track` on `ShashlikMap` (`map/src/lib.rs`)

Two methods added directly to `ShashlikMap`, at the same level as `clear_routes` and `load_kml_path`.
No `TrackController` wrapper — unlike routing, there is no async work, no retry, no alternatives
counter, and no warm-up concern. The logic is:

```rust
pub fn draw_track(&mut self, lon_lats: Vec<(f64, f64)>) {
    self.renderer.api().clear_render_groups(HashSet::from(["track".to_string()]));
    let converter = self.create_location_coord_converter();
    let points: Vec<Point> = lon_lats.iter()
        .map(|(lon, lat)| converter(&Point::new(*lon, *lat)))
        .collect();
    let group = TrackGroup::new(points);
    self.renderer.api().add_render_group(
        "track".to_string(),
        SpatialData::transform(group.first_point()),
        Box::new(group),
    );
}

pub fn clear_track(&mut self) {
    self.renderer.api().clear_render_groups(HashSet::from(["track".to_string()]));
}
```

Note: `clear_render_groups` before `add_render_group` is required for replace semantics.
The `spatial_data_map` entry is overwritten by key, but GPU layer buffers are not automatically
evicted — explicit clearing is necessary (same pattern as `RouteController::clear_routes`).

---

### Step 4 — uniffi surface (`ffi-run/src/lib.rs`)

Add a `LatLon` record and two exported methods:

```rust
#[derive(uniffi::Record)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

#[uniffi::export]
impl ShashlikMapApi {
    fn draw_track(&self, points: Vec<LatLon>) {
        let mut map = self.shashlik_map.write().unwrap();
        let lon_lats: Vec<(f64, f64)> = points.iter().map(|p| (p.lon, p.lat)).collect();
        map.draw_track(lon_lats);
    }

    fn clear_track(&self) {
        let mut map = self.shashlik_map.write().unwrap();
        map.clear_track();
    }
}
```

uniffi generates the Kotlin data class `LatLon` and the two methods automatically.

---

## What this does NOT change

- `renderer-common` — no new feature layer tag
- `renderer-gpu` — no pipeline or pass node changes
- `app-surface`, `winit-run`, `kmp` — no changes
- External `osm`/`shashlik-tiles-gen-v0` crate — no style loader change needed

---

## Live trail usage pattern

The API supports incremental live trail by design. The caller accumulates points and replaces
the track each GPS update:

```kotlin
val trackPoints = mutableListOf<LatLon>()

fun onGpsUpdate(lat: Double, lon: Double) {
    trackPoints.add(LatLon(lat, lon))
    shashlikMapApi?.drawTrack(trackPoints)
}
```

Re-tessellation of a typical GPS track (hundreds of points) takes microseconds on the CPU side.
At 1 Hz this is negligible.
