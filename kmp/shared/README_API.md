# Shashlik Map SDK - Shared Module API Reference

This document serves as a comprehensive reference guide for the public API exposed by the `:shared` module of the Shashlik Map Kotlin Multiplatform (KMP) component. It is intended to help developers and AI agents understand, consume, and maintain the API effectively.

<!-- BEGIN GENERATED: version -->
This document describes **mapshared 0.3.21**.
<!-- END GENERATED: version -->

If the version you resolved differs from the one above, treat this document as
unreliable. If the compiler disagrees with it, the compiler is right: stop and
tell the user rather than working around it.

---

## Build facts

<!-- BEGIN GENERATED: facts -->
| | |
|---|---|
| Coordinates | `io.github.shashlikmap:mapshared:0.3.21` |
| Repository | `mavenCentral()` |
| Platforms | **Android only** — iOS targets are not built or published |
| Kotlin targets | `android` |
| Android minSdk | 26 |
| Android compileSdk | 36 |
| JVM target | 11 |
| Native ABIs | `arm64-v8a` |
| Manifest permissions | `ACCESS_COARSE_LOCATION`, `ACCESS_FINE_LOCATION`, `INTERNET` |
<!-- END GENERATED: facts -->

---

## Setup

**Required from 0.3.21.** The SDK declares `org.rustls:rustls-platform-verifier`
in its POM. That artifact is not on Maven Central, Google, JitPack or Sonatype —
rustls publishes the Android support library from a Maven archive in their own
repository, so consumers must add it explicitly:

```kotlin
// settings.gradle.kts
dependencyResolutionManagement {
    repositories {
        mavenCentral()
        maven("https://github.com/rustls/rustls-platform-verifier/raw/maven-archive/android-release-support/maven/") {
            name = "RustlsAndroidSupport"
            content { includeGroup("org.rustls") }
        }
    }
}
```

Without it the build fails to resolve `org.rustls:rustls-platform-verifier`.

Never choose this version yourself. The support library must stay SemVer-compatible
with the Rust crate inside `libffi_run.so`; a mismatch crashes at runtime instead
of failing resolution. 0.3.21 pins `0.2.0`, read from the
`rustls-platform-verifier-android` entry in `Cargo.lock`.

Versions before 0.3.21 needed none of this — the verifier was a JNI method inside
`libffi_run.so` with no Maven coordinate.

Also required: `mavenCentral()`, `shashlikMapInit()` in `Application.onCreate()`,
and an `Activity` host. Location permissions are declared and requested by the SDK
itself.

---

## Known limitations

Verified against 0.3.21. If you need something listed here, it does not exist yet —
tell the user rather than reaching for an undocumented API.

### Broken or disabled

- **`ShashlikShape` `anchor` does not move an existing shape.** The `updateShape`
  call is commented out (`Overlay.kt:128`) and `DisposableEffect` keys on `anchor`,
  so changing it destroys and recreates the shape. Do not rely on cheap live
  anchor updates, and do not write examples that imply them.
- **No iOS artifact.** iOS targets are commented out
  (`kmp/shared/build.gradle.kts:57`). Android only, `arm64-v8a` only.
- **Location permission revocation is not handled.** `SimpleLocationManager.start()`
  is annotated `@SuppressLint("MissingPermission")` (`SimpleLocationManager.kt:38`);
  revoking permission while the map runs is untested.

### Not supported yet

Nothing outside the inventory above exists. Specifically:

- **No tap callback carrying map coordinates.** `Modifier.mapGestures` takes
  `onGesture: () -> Unit` — it reports only *that* a gesture happened, with no
  position and no gesture type.
- **Gestures are opt-in.** Pan, zoom and pitch only work if you apply
  `Modifier.mapGestures()` yourself; `ShashlikMap` does not add it for you.
- **No custom marker icons and no style API.**
- **`LineShape` fails silently** when given fewer than two distinct points — it
  draws nothing and reports nothing (`Overlay.kt:66`).
- **Tiles cover Japan and the SF Bay Area only.** Test with coordinates there.
- The Android emulator may need GPU mode `Software`, and debug builds are much
  slower than release.

---

## Public API inventory

<!-- BEGIN GENERATED: inventory -->
Derived from `api/shared.api`. A name here with no section in this
document means the document is incomplete. The reverse does **not**
hold: this is not an exhaustive list of supported API (see the note
at the end), so never delete a section merely because it is absent here.

**Top-level functions**
- `ConvexPolygon`
- `LineShape`
- `ShashlikMap`
- `ShashlikShape`
- `isDebugBuild`
- `mapGestures`
- `rememberLocationState`
- `shashlikMapInit`

**Extension properties** — call these as properties, not functions.
They appear in `shared.api` as JVM `getX`/`setX` accessors.
- `width`

**Types**
- `LocationState`
- `ShashlikMapApiHolder`

Not listed: `uniffi.ffi_run` types (`Point`, `Color`, `ShapeType`,
`ShashlikMapApi`) are excluded from `shared.api`, so their absence here
does **not** mean they are unsupported. See Known limitations.
<!-- END GENERATED: inventory -->

---

## 🔗 Project Metadata & Repository Links
- **Official Repository**: [https://github.com/ShashlikMap/shashlik-map](https://github.com/ShashlikMap/shashlik-map)
- **Demo Module Source**: [https://github.com/ShashlikMap/shashlik-map/tree/main/kmp/demo](https://github.com/ShashlikMap/shashlik-map/tree/main/kmp/demo)

---

## 📦 Project Modules

### 1. `:shared` Module
Contains the core multiplatform business logic, location management abstractions, and the interface mapping to the underlying Rust WGPU map rendering engine.

### 2. `:composeApp` (Demo Module)
A functional usage showcase located inside `kmp/demo`. It serves as a ready-to-run reference sandbox implementing interactive gestures (scroll, pan, zoom, pitch), route calculations via `ShashlikMapApiHolder`, and toggle buttons for features like vector tiles (MVT) or camera configurations.

---

## Table of Contents
1. [Core Components](#1-core-components)
2. [State Hoisting & Management](#2-state-hoisting--management)
3. [Map Overlays & Shapes](#3-map-overlays--shapes)
4. [Low-Level API Access](#4-low-level-api-access)
5. [Platform Specific Initialization](#5-platform-specific-initialization)

---

## 1. Core Components

### `ShashlikMap`
The main visual entry point for the map component. It integrates the underlying Rust-powered WGPU rendering engine within a responsive Jetpack Compose container.

```kotlin
@Composable
fun ShashlikMap(
    modifier: Modifier = Modifier,
    state: LocationState = rememberLocationState(),
    withPuck: Boolean = true,
    withAutoLocationEvent: Boolean = true,
    mvtTiles: Boolean = false,
    followModeEnabled: Boolean = true,
    camAnimationEnabled: Boolean = true,
    puckAnimationEnabled: Boolean = true,
    content: @Composable () -> Unit = {}
)
```

#### Parameters:
- **`modifier`**: The `Modifier` to be applied to the map layout container.
- **`state`**: The hoisted `LocationState` governing the location coordinates and bearing of the marker.
- **`withPuck`**: When `true`, displays a location marker puck at the current coordinates.
- **`withAutoLocationEvent`**: Automatically listens to and updates the user's current GPS location.
- **`mvtTiles`**: Activates Mapbox Vector Tile (MVT) rendering mode when set to `true`.
- **`followModeEnabled`**: When `true`, automatically locks and centers the map camera on the location puck.
- **`camAnimationEnabled`**: Enables smooth transitions and fluid animations for camera view changes.
- **`puckAnimationEnabled`**: Enables smooth positional interpolations for the location puck.
- **`content`**: Composable lambda slot to draw overlays/shapes (`ConvexPolygon`, `LineShape`) directly on top of the map layer.

### `Modifier.mapGestures`
An extension modifier that attaches interactive gesture detection (single-finger pan/drag, two-finger pinch zoom, two-finger scroll pitch) directly to the map container.

```kotlin
fun Modifier.mapGestures(onGesture: () -> Unit = {}): Modifier
```

#### Parameters:
- **`onGesture`**: Optional callback lambda triggered whenever a user gesture is detected (useful for disabling automatic camera follow mode during user interaction).

#### Quick Start Usage Example:
```kotlin
ShashlikMap(
    state = rememberLocationState(latitude = 35.6879, longitude = 139.7570),
    withPuck = true
) {
    // Convex polygon centered at Tokyo with a 10.dp radius
    ConvexPolygon(
        center = Point(x = 139.7570, y = 35.6879),
        radius = 10.dp,
        sides = 5,
        color = Color.Red
    )

    // Polyline connecting geographic coordinates
    LineShape(
        points = listOf(
            Point(x = 139.75, y = 35.68),
            Point(x = 139.76, y = 35.69)
        ),
        color = Color.Blue,
        width = 2f
    )
}
```

### `isDebugBuild`
A multiplatform build flag property (`expect val isDebugBuild: Boolean`) that returns `true` when running on a debug build variant.

---

## 2. State Hoisting & Management

### `LocationState`
A highly observable, stable state container designed to hoist positional information up to the application level.

```kotlin
@Stable
class LocationState(
    latitude: Double = 0.0,
    longitude: Double = 0.0,
    bearing: Float? = null
) {
    var latitude: Double by mutableStateOf(latitude)
    var longitude: Double by mutableStateOf(longitude)
    var bearing: Float? by mutableStateOf(bearing)
}
```

### `rememberLocationState`
Convenience Composable method to safely create and remember `LocationState` across recompositions and system configuration adjustments (such as display rotations). Built on top of `rememberSaveable`.

```kotlin
@Composable
fun rememberLocationState(
    latitude: Double = 0.0,
    longitude: Double = 0.0,
    bearing: Float? = null
): LocationState
```

---

## 3. Map Overlays & Shapes

These Composables must be declared inside the `content` lambda block of a `ShashlikMap` declaration. They leverage `DisposableEffect` to strictly bind overlay lifecycles with the corresponding Composable lifecycle.

### `ConvexPolygon`
Draws a completely filled convex polygon on the map layer centered at a specific geographic point.

```kotlin
@Composable
fun ConvexPolygon(
    center: uniffi.ffi_run.Point,
    radius: androidx.compose.ui.unit.Dp,
    sides: Int,
    color: androidx.compose.ui.graphics.Color
)
```

#### Parameters:
- **`center`**: The geographic center anchor point of the polygon.
- **`radius`**: The distance from the center to each vertex as a `Dp` value.
- **`sides`**: The number of sides (vertices) of the polygon. Must be at least 3.
- **`color`**: The color used to fill the polygon.

### `LineShape`
Draws a connected path overlay passing through an array of geographic coordinates.

```kotlin
@Composable
fun LineShape(
    points: List<uniffi.ffi_run.Point>,
    color: androidx.compose.ui.graphics.Color,
    width: Float = 1f
)
```

#### Parameters:
- **`points`**: The list of geographic points defining the path of the line.
- **`color`**: The color of the line.
- **`width`**: The width of the line. *(Note: abstract unit at this moment; a resolution-independent coordinate unit will be provided in a future iteration).*

### `ShashlikShape`
Low-level component managing underlying shapes. Handles automatic shape instantiation on addition and resource cleanup on disposal.

```kotlin
@Composable
fun ShashlikShape(
    points: List<uniffi.ffi_run.Point>,
    anchor: uniffi.ffi_run.Point?,
    type: uniffi.ffi_run.ShapeType,
    color: androidx.compose.ui.graphics.Color
)
```

#### Parameters:
- **`points`**: The points defining the shape. If `anchor` is `null`, `points` are interpreted as geographic coordinates. If `anchor` is provided, `points` are interpreted as relative offset points in dp from the `anchor`.
- **`anchor`**: Optional geographic anchor point for the shape.
- **`type`**: The type of shape to render (`ShapeType.Polygon` or `ShapeType.Line`).
- **`color`**: The color of the shape.

### `ShapeType.Line.width`
Extension property providing convenient access to the width value of a `ShapeType.Line` instance, falling back to default `1f` if unset.

```kotlin
val ShapeType.Line.width: Float
```

---

## 4. Low-Level API Access

### `ShashlikMapApiHolder`
A global singleton holder offering access to the low-level rust FFI bindings (`ShashlikMapApi`) after successful layout initialization.

```kotlin
object ShashlikMapApiHolder {
    var shashlikMapApi: ShashlikMapApi?
}
```

---

## 5. Platform Specific Initialization

### `shashlikMapInit` (Android Only)
Instantiates global logging tree hook (`Timber`) and other Android context defaults. Should be invoked inside `Application.onCreate()`.

```kotlin
fun shashlikMapInit()
```
