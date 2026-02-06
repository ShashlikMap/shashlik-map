# Shashlik Map
A WIP map engine written in Rust using WGPU.

The initial goal of the project is to learn a new cross-platform tech stack to build a mobile-ready MapEngine
with focus on Navigation features(including DeadReckoning and Map-matching)

## Showcases
Running on macOS, Android and iOS

<img width="450" alt="Screenshot 2026-02-06 at 08 56 25" src="https://github.com/user-attachments/assets/b9a734d5-38f6-494c-8793-799376d392e7" />
<img width="130" height="1872" alt="Screenshot_20260206_085918" src="https://github.com/user-attachments/assets/eabb3468-6206-4dc9-b243-73bc34ce0dff" />
<img width="130" height="2622" alt="Simulator Screenshot - iPhone 16 Pro - 2025-12-24 at 11 46 18" src="https://github.com/user-attachments/assets/ed5a0121-1402-40a6-ab26-eb0c39853708" />

## Tech stack
The stack leverages the following approaches and libraries:
- Map vector graphics renderer written in Rust using [WGPU](https://github.com/gfx-rs/wgpu) as a low-level cross-platform graphics API and
with [RustyBuzz](https://github.com/harfbuzz/rustybuzz) support as a vector font shaper for TextRenderer.
- Uses custom tiles, a simple tiles generator and a tile server, [separate repo](https://github.com/ShashlikMap/shashlik-tiles-gen-v0). The server is running in free AWS EC2 Cloud. 
- Kotlin/Compose Multiplatfom, [uniffi-rs](https://github.com/mozilla/uniffi-rs) and [gobley](https://github.com/gobley/gobley) projects enable fast and seamless integration 
with Android/iOS mobile apps(Android is priority for now)
- [Rust Valhalla client](https://github.com/jelmer/valhalla-client-rs) is used a routing clieng/engine

### The important component diagram:
<img width="500" alt="ShashlikDiagram" src="https://github.com/user-attachments/assets/c0e6d330-2e97-4f77-acba-e7b186fcb194" />

## Roadmap
### Completed
- [x] Create a baseline POC with initial architecture
- [x] Complete README and examples
- [x] Initial rendering to texture
- [x] Better integration with [SlintUI](https://slint.dev/blog/slint-1.12-released)
### Now
- [ ] _In progress_ General Renderer refactoring
- [ ] _In progress_ "Circles" line style
### Next
- [ ] SSAO(Screen Space Ambient Occlusion)
- [ ] Implement an initial geometric Map-matching POC
- [ ] CI for KMP mobile SDK + Screenshot rendering
- [ ] Integrate a simple search
### Later
- [ ] Support Mapbox [tilesets](https://docs.mapbox.com/data/tilesets/guides/vector-tiles-standards/)
- [ ] Software Dead-reckoning
- [ ] Complete iOS counter-part
- [ ] Move TextRenderer to the separate repo

## Running examples
### macOS
In root folder:
```
cargo run --package winit-run --release
```
### Android
- Make sure the latest Xcode is installed!
- Open "kmp" folder in AndroidStudio and just Run "demo" app or execute:
```
./gradlew :composeApp:installRelease && adb shell am start -n "com.shashlik.demo/com.shashlik.demo.MainActivity"
```
### iOS
Open "kmp/iosApp" project in XCode and just Run it

## Integration with KMP apps
1. Add dependency to the version catalog

```
[versions]
shashlikMap = "0.2.1"

[libraries]
shashlikmap = { module = "io.github.shashlikmap:mapshared", version.ref = "shashlikMap" }
```
In build.gradle.kts(KMP or Android):
```
implementation(libs.shashlikmap)
```
2. Include Composable function `ShashlikMap { _, _ -> }` anywhere in your Compose UI
```kotlin
   @Composable
   fun App() {
       MaterialTheme {
           ShashlikMap { _, _ -> }
       }
   }
```
- Note: Android app will ask for locations permissions.

## Known issues
- Tileset on the Web Service is generated only for Japan(Kanto region) and USA(Bay Area)
- Android app might not work on Android Emulator with hardware GPU acceleration. Try to change GPU mode to `Software` one.
- Debug build performance is significantly lower than Release build.
