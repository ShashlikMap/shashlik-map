use cgmath::{Deg, Matrix4, Vector3};

#[derive(Clone)]
#[derive(Debug)]
pub struct SpatialData {
    pub transform: Vector3<f64>,
    pub scale: f64,
    pub yaw: f64,
    pub size: (f64, f64),
    pub normal_scale: f64,
    pub sk: i32
}

impl SpatialData {
    pub fn new() -> SpatialData {
        SpatialData {
            transform: Vector3::new(0.0, 0.0, 0.0),
            scale: 1.0,
            yaw: 0.0,
            size: (0.0, 0.0),
            normal_scale: 1.0,
            sk: -1
        }
    }

    pub fn transform(transform: Vector3<f64>) -> SpatialData {
        SpatialData { transform, scale: 1.0, yaw: 0.0, size: (0.0, 0.0), normal_scale: 1.0, sk: -1 }
    }

    pub fn size(mut self, size: (f64, f64)) -> SpatialData {
        self.size = size;
        self
    }

    pub fn scale(&mut self, scale: f64) {
        self.scale = scale;
    }
    pub fn yaw(&mut self, yaw: f64) {
        self.yaw = yaw;
    }

    pub fn normal_scale(&mut self, normal_scale: f64) {
        self.normal_scale = normal_scale;
    }

    pub fn scale_rot_matrix(&self) -> Matrix4<f64> {
        let scale_matrix = Matrix4::<f64>::from_scale(self.scale);
        let rotation_matrix = Matrix4::<f64>::from_angle_z(Deg(self.yaw));
        scale_matrix * rotation_matrix
    }
}
