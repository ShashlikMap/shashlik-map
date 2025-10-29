#[cfg(all(target_os = "android"))]
mod android;

#[cfg(all(target_os = "ios"))]
pub mod ios;