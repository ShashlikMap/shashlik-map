use cgmath::Vector3;

#[derive(Clone)]
#[derive(Debug)]
pub struct SpatialData {
    pub transform: Vector3<f64>,
    pub scale: f64,
    pub size: (f64, f64),
}

impl SpatialData {
    pub fn new() -> SpatialData {
        SpatialData {
            transform: Vector3::new(0.0, 0.0, 0.0),
            scale: 1.0,
            size: (0.0, 0.0),
        }
    }

    pub fn transform(transform: Vector3<f64>) -> SpatialData {
        SpatialData { transform, scale: 1.0, size: (0.0, 0.0) }
    }

    pub fn size(mut self, size: (f64, f64)) -> SpatialData {
        self.size = size;
        self
    }

    pub fn scale(&mut self, scale: f64) {
        self.scale = scale;
    }
}
