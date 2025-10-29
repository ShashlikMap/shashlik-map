use cgmath::Vector3;

#[derive(Clone)]
#[derive(Debug)]
pub struct SpatialData {
    pub transform: Vector3<f32>,
}

impl SpatialData {
    pub fn new() -> SpatialData {
        SpatialData {
            transform: Vector3::new(0.0, 0.0, 0.0),
        }
    }

    pub fn transform(transform: Vector3<f32>) -> SpatialData {
        SpatialData { transform }
    }
}
