#[cfg(feature = "linux-cpu")]
mod main_cpu;
#[cfg(feature = "linux-cpu")]
use crate::main_cpu::main_internal;

#[cfg(any(feature = "linux-gpu", target_os = "macos"))]
mod main_gpu;
#[cfg(any(feature = "linux-gpu", target_os = "macos"))]
use crate::main_gpu::main_internal;

fn main() {
    main_internal()
}
