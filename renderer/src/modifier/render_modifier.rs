use glam::{DMat4, DVec3};

#[derive(Clone)]
#[derive(Debug)]
pub struct SpatialData {
    pub transform: DVec3,
    pub scale: DVec3,
    pub yaw: f64,
    pub size: (f64, f64),
}

impl SpatialData {
    pub fn new() -> SpatialData {
        SpatialData {
            transform: DVec3::new(0.0, 0.0, 0.0),
            scale: DVec3::splat(1.0),
            yaw: 0.0,
            size: (0.0, 0.0),
        }
    }

    pub fn transform(transform: DVec3) -> SpatialData {
        SpatialData { transform, scale: DVec3::splat(1.0), yaw: 0.0, size: (0.0, 0.0) }
    }

    pub fn size(mut self, size: (f64, f64)) -> SpatialData {
        self.size = size;
        self
    }

    pub fn scale(&mut self, scale: DVec3) {
        self.scale = scale;
    }
    pub fn yaw(&mut self, yaw: f64) {
        self.yaw = yaw;
    }
    
    pub fn scale_rot_matrix(&self) -> DMat4 {
        let scale_matrix = DMat4::from_scale(self.scale);
        let rotation_matrix = DMat4::from_rotation_z(self.yaw.to_radians());
        scale_matrix * rotation_matrix
    }
}
