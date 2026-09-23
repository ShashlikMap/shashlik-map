# Shashlik Map SDK - Shared Module API Reference

This document serves as a comprehensive reference guide for the public API exposed by the `:shared` module of the Shashlik Map Kotlin Multiplatform (KMP) component. It is intended to help developers and AI agents understand, consume, and maintain the API effectively.

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
- **`state`**: The hoisted `LocationState` governing the location coordinates and bearing of the marker.
- **`withPuck`**: When `true`, displays a location marker puck at the current coordinates.
- **`withAutoLocationEvent`**: Automatically listens to and updates the user's current GPS location.
- **`mvtTiles`**: Activates Mapbox Vector Tile (MVT) rendering mode when set to `true`.
- **`followModeEnabled`**: When `true`, automatically locks and centers the map camera on the location puck.
- **`camAnimationEnabled`**: Enables smooth transitions and fluid animations for camera view changes.
- **`puckAnimationEnabled`**: Enables smooth positional interpolations for the location puck.
- **`content`**: Composable lambda slot to draw overlays/shapes (`ConvexPolygon`, `LineShape`) directly on top of the map layer.

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
- **`points`**: The list of geographic points in Mercator coordinates defining the path of the line.
- **`color`**: The color of the line.
- **`width`**: The width of the line. *(Note: abstract unit at this moment; a resolution-independent coordinate unit will be provided in a future iteration).*

### `ShashlikShape`
Low-level component managing underlying shapes. Handles automatic shape instantiation on addition, dynamic anchor position updates via `updateShape`, and resource cleanup on disposal.

```kotlin
@Composable
fun ShashlikShape(
    points: List<uniffi.ffi_run.Point>,
    anchor: uniffi.ffi_run.Point?,
    type: uniffi.ffi_run.ShapeType,
    color: uniffi.ffi_run.Color
)
```

#### Parameters:
- **`points`**: The points defining the shape. If `anchor` is `null`, `points` are interpreted as Mercator coordinates. If `anchor` is provided, `points` are interpreted as relative offset points in dp from the `anchor`.
- **`anchor`**: Optional geographic anchor point. When provided, changes to `anchor` dynamically update the shape's position on the map without re-creating the underlying shape.
- **`type`**: The type of shape to render (`ShapeType.Polygon` or `ShapeType.Line`).
- **`color`**: The color of the shape.

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
