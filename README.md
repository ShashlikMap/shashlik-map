# Shashlik Map
A WIP map engine built in Rust using WGPU.

The first goal of the project is to create a MVP that can be used for walking and vehicle navigation.

## Features
* Runs on macOS, Android and iOS
  * Uses [uniffi-rs](https://github.com/mozilla/uniffi-rs) to provide Rust<->Kotlin/Swift bindings
* Renders a custom vector tiles from the simplest tile server.
  * Support remote and local fetching

## Showcase
<img width="500" alt="Screenshot 2025-10-30 at 21 31 00" src="https://github.com/user-attachments/assets/0af3afd1-fd61-4db5-8d39-0e8befecae58" />
<img width="500" alt="Screenshot 2025-10-30 at 21 31 21" src="https://github.com/user-attachments/assets/a5606a78-2d4b-4b42-9b7c-4c9a612688e5" />

