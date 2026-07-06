# Shashlik Map
A WIP map engine written in Rust using WGPU.

The initial goal of the project is to learn a new cross-platform tech stack to build a mobile and Linux KMS-ready map engine. 
The project focuses on rendering and navigation features, including dead reckoning and map matching.

I'm writing about the tech I've learned [here](https://hackmd.io/@agent10)

## Showcases
Running on macOS, Android, iOS and Linux via KMS

<img width="450" alt="613070008-6cb9f503-2145-41b3-85af-e274d27bfee9" src="https://github.com/user-attachments/assets/ea463fe8-6cc5-49c3-bb4e-b4b8a697b66c" />
<img width="130" alt="613070241-59d29a03-5443-43f6-9dd1-7a451ee8ef89" src="https://github.com/user-attachments/assets/996a4970-0103-4336-84fa-2e82fda39561" />
<img width="450" alt="613070156-10d8f87c-6072-440c-a169-0647b0e88dad" src="https://github.com/user-attachments/assets/16aa69df-ea92-4e0a-9521-635bee83e4dd" />


## Tech stack
The stack leverages the following approaches and libraries:
- Map vector graphics renderer written in Rust using [WGPU](https://github.com/gfx-rs/wgpu) as a low-level cross-platform graphics API and
with [RustyBuzz](https://github.com/harfbuzz/rustybuzz) support as a vector font shaper for TextRenderer.
- Uses custom tiles, a simple tiles generator and a tile server, [separate repo](https://github.com/ShashlikMap/shashlik-tiles-gen-v0). The server is running in free AWS EC2 Cloud. 
- Kotlin/Compose Multiplatfom, [uniffi-rs](https://github.com/mozilla/uniffi-rs) and [gobley](https://github.com/gobley/gobley) projects enable fast and seamless integration 
with Android/iOS mobile apps(Android is priority for now)
- [Slint UI](https://github.com/slint-ui/slint) is used for native platforms (macOS and Linux)
- [Rust Valhalla client](https://github.com/jelmer/valhalla-client-rs) is used a routing clieng/engine

### The component diagram:
<img width="500" alt="ShashlikDiagram" src="https://github.com/user-attachments/assets/c0e6d330-2e97-4f77-acba-e7b186fcb194" />

## Roadmap
### Completed
- [x] Create a baseline POC with initial architecture
- [x] Complete README and examples
- [x] Initial rendering to texture
- [x] Better integration with [SlintUI](https://slint.dev/blog/slint-1.12-released)
- [x] A GPU-driven dotted line rendering
- [x] Initial SSAO(Screen Space Ambient Occlusion)
- [x] Running on pure Linux via KMS with Slint UI
- [x] Simple shadow mapping
- [x] Integration with WESL
- [x] Initial MVT tiles support

### Now
- [ ] _In progress_ General Renderer refactoring
### Next
- [ ] Initial CPU renderer
- [ ] Integrate a simple search
### Later
- [ ] Move the custom tile generator to the cloud
- [ ] Implement an initial geometric Map-matching POC
- [ ] Software Dead-reckoning
- [ ] CI for KMP mobile SDK + Screenshot rendering
- [ ] Move TextRenderer to the separate repo
- [ ] Explore Mesh Shader
- [ ] Post-processing AA(SMAA)

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

### Linux via KMS
The current version has been tested on a Raspberry Pi 4 with Raspberry Pi OS Lite.
It works via KMS and doesn't require a window subsystem.

Prerequisites for the Linux device:
- Enable SSH
- Install and configure Vulkan:

`sudo apt install mesa-vulkan-drivers vulkan-tools libvulkan1`

- Install additional required libraries:

`sudo apt install libfontconfig1 libgbm1 libinput10 libxkbcommon-x11-0`

Prerequisites for the building machine:
- Install [cross-rs](https://github.com/cross-rs/cross)
  `cargo install cross --git https://github.com/cross-rs/cross --branch main --force`
- Install Docker

Execute *kms_deploy.sh* script:
- `chmod +x kms_deploy.sh`
- `TARGET_HOST=admin@raspberrypi.local ./kms_deploy.sh`. Note: Replace with your actual device user and address.

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
- Tileset on the Web Service is generated only for Japan and USA(Bay Area)
- Android app might not work on Android Emulator with hardware GPU acceleration. Try to change GPU mode to `Software` one.
- Debug build performance is significantly lower than Release build.
