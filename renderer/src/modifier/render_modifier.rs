use cgmath::Vector3;

#[derive(Clone)]
#[derive(Debug)]
pub struct SpatialData {
    pub transform: Vector3<f64>,
    pub scale: f64,
    pub yaw: f32,
    pub size: (f64, f64),
    pub normal_scale: f32,
}

impl SpatialData {
    pub fn new() -> SpatialData {
        SpatialData {
            transform: Vector3::new(0.0, 0.0, 0.0),
            scale: 1.0,
            yaw: 0.0,
            size: (0.0, 0.0),
            normal_scale: 1.0
        }
    }

    pub fn transform(transform: Vector3<f64>) -> SpatialData {
        SpatialData { transform, scale: 1.0, yaw: 0.0, size: (0.0, 0.0), normal_scale: 1.0 }
    }

    pub fn size(mut self, size: (f64, f64)) -> SpatialData {
        self.size = size;
        self
    }

    pub fn scale(&mut self, scale: f64) {
        self.scale = scale;
    }
    pub fn yaw(&mut self, yaw: f32) {
        self.yaw = yaw;
    }

    pub fn normal_scale(&mut self, normal_scale: f32) {
        self.normal_scale = normal_scale;
    }
}
