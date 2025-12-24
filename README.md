# Shashlik Map
A WIP map engine written in Rust using WGPU.

The initial goal of the project is to learn a new cross-platform tech stack to build a mobile-ready MapEngine
with focus on Navigation features(including DeadReckoning and Map-matching)

## Showcases
Running on macOS, Android and iOS

<img width="400" alt="Screenshot 2025-12-24 at 11 21 52" src="https://github.com/user-attachments/assets/ce9b5ea2-9cbc-40e1-b8cf-455c80f47b22" />
<img width="150" height="1872" alt="Screenshot_20251224-112013" src="https://github.com/user-attachments/assets/2f3c26a2-5ffc-46a1-9616-eefc59df12a0" />
<img width="150" height="2622" alt="Simulator Screenshot - iPhone 16 Pro - 2025-12-24 at 11 46 18" src="https://github.com/user-attachments/assets/ed5a0121-1402-40a6-ab26-eb0c39853708" />

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
### Now
- [x] Create a baseline POC with initial architecture
- [ ] Complete README and examples
- [ ] Location sharing from GoogleMaps to improve a "field" testing
### Next
- [ ] Implement an initial geometric Map-matching POC
- [ ] General Renderer refactoring
- - [ ] Support a texture as a render target to improve CI and integration with [SlintUI](https://slint.dev/blog/slint-1.12-released)
- [ ] CI for KMP mobile SDK
### Later
- [ ] Software Dead-reckoning
- [ ] Move TextRenderer to the separate repo
- [ ] Support Mapbox [tilesets](https://docs.mapbox.com/data/tilesets/guides/vector-tiles-standards/)
- [ ] Complete iOS counter-part
- [ ] Integrate a simple search

## Running examples
### macOS
In root folder:
```
cargo run --package winit-run --release
```
### Android
Open "kmp" folder in AndroidStudio and just Run "demo" app or execute:
```
./gradlew :composeApp:installRelease && adb shell am start -n "com.shashlik.demo/com.shashlik.demo.MainActivity"
```
### iOS
Open "kmp/iosApp" project in XCode and just Run it

## Integration to mobile apps
TODO
