use num::clamp;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) struct Transition2d3dHelper {
    scale_2d_3d: f32,
    zero_zoom_level_loaded: Arc<AtomicBool>,
}

impl Transition2d3dHelper {
    pub(crate) fn new(zero_zoom_level_loaded: Arc<AtomicBool>) -> Self {
        Transition2d3dHelper {
            scale_2d_3d: 0.0,
            zero_zoom_level_loaded,
        }
    }

    pub fn update(&mut self, level: f32, anim_speed: f32) -> f32 {
        let scale_2d_3d_mul = if level >= 1.8 {
            -1.0
        } else if self.zero_zoom_level_loaded.load(Ordering::Relaxed) {
            1.0
        } else {
            0.0
        };

        self.scale_2d_3d = clamp(self.scale_2d_3d + scale_2d_3d_mul * anim_speed, 0.0, 1.0);
        self.scale_2d_3d
    }
    
    pub fn scale_2d_3d(&self) -> f32 { self.scale_2d_3d }
}
