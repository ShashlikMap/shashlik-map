# iOSDemoApp

An iOS demo aiming for a feature‑parity port of `AndroidDemoApp`. We are in Step 3: integrating Rust + UniFFI with a Metal-backed WGPU renderer.

## Current Architecture
- SwiftUI entry (`iOSDemoAppApp.swift` → `ContentView` → `MapWithControlsView`).
- `MetalMapUIView` (UIKit) hosts a `CAMetalLayer` and drives a render loop via `CADisplayLink`, calling `ShashlikMapApi.render()` each frame.
- Rust exported API (via UniFFI): `create_shashlik_map_api`, `render`, `temp_external_input` (temporary test input hook).
- Generated artifacts (NOT in git): `ffi_run.xcframework`, `ffi_run.swift`, header + modulemap under `iOSDemoApp/generated/{lib,swift}`.

## Project Structure
The project is organized with a clear separation of platform-specific code:

```
shashlik-wgpu/
├── ffi-run/                      # Rust FFI library
│   └── src/
│       ├── lib.rs                # Platform-independent core
│       └── platform/
│           └── ios.rs            # iOS-specific implementation
├── iOSDemoApp/                   # iOS Swift application
│   └── generated/                # Auto-generated bindings (not in git)
│       ├── lib/
│       │   └── ffi_run.xcframework
│       └── swift/
│           ├── ffi_run.swift
│           ├── ffi_runFFI.h
│           └── ffi_runFFI.modulemap
└── temp_ios_ffi.sh               # Build script for iOS bindings
```

All generated files are now consolidated in the `iOSDemoApp/generated/` directory, which simplifies integration and ensures consistency across builds.

## UniFFI / XCFramework Regeneration
Automatic on build (Run Script phase) unless you set `SKIP_UNIFFI_GEN=1`.

Manual invocation:
```zsh
# From project root:
bash shashlik-wgpu/temp_ios_ffi.sh                    # debug build
FORCE_UNIFFI_GEN=1 bash shashlik-wgpu/temp_ios_ffi.sh # force rebuild ignoring timestamps
```

All generated artifacts are consistently placed in the consolidated output directory:
```
shashlik-wgpu/iOSDemoApp/generated/lib/ffi_run.xcframework
shashlik-wgpu/iOSDemoApp/generated/swift/ffi_run.swift
shashlik-wgpu/iOSDemoApp/generated/swift/ffi_runFFI.h
shashlik-wgpu/iOSDemoApp/generated/swift/ffi_runFFI.modulemap
```

The build script includes safety measures to maintain consistency:
- Timestamp-based incremental builds to avoid unnecessary regeneration
- Automatic backup and fallback for Swift bindings if generation fails
- Environment variables to control build behavior (FORCE_UNIFFI_GEN, SKIP_UNIFFI_GEN)

## Tiles.db Location (Recommended)
Inside the app sandbox:
```
Library/Application Support/ShashlikTiles/Tiles.db
```
The Swift view helper `MetalMapView.swift` uses `defaultTilesDbPath()` to construct this path and ensure the directory exists before passing it to Rust.

### Supplying Tiles.db to the iOS Simulator
You must copy the host database into the simulator’s app container (each install/device has a unique UUID). Use:
```zsh
SIM_DATA=$(xcrun simctl get_app_container booted com.shashlik.demo.ios data)
HOST_DB="$HOME/Library/Application Support/ShashlikTiles/Tiles.db"
DEST_DIR="$SIM_DATA/Library/Application Support/ShashlikTiles"
mkdir -p "$DEST_DIR"
cp -v "$HOST_DB" "$DEST_DIR/Tiles.db"
ls -lh "$DEST_DIR/Tiles.db"
```