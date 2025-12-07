use cgmath::{Basis3, Deg, InnerSpace, Matrix4, Point3, Rad, Rotation, Rotation3, SquareMatrix, Vector2, Vector3};

pub struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
    fovy: f32,
    znear: f32,
    zfar: f32,
    perspective_matrix: Matrix4<f32>,
}

impl Camera {
    pub fn new() -> Self {
        Camera {
            eye: (0.0, 0.0, 200.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            fovy: 45.0,
            znear: 1.0,
            zfar: 2000000.0,
            perspective_matrix: Matrix4::identity(),
        }
    }

    pub fn build_view_projection_matrix(&mut self) -> cgmath::Matrix4<f64> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        (self.perspective_matrix * view).cast().unwrap()
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let aspect = width as f32 / height as f32;
        self.perspective_matrix =
            cgmath::perspective(cgmath::Deg(self.fovy), aspect, self.znear, self.zfar);
    }
}

pub struct CameraController {
    #[allow(dead_code)]
    speed: f32,
    pub zoom_delta: f32,
    pub pan_delta: Vector2<f32>,
    pub camera_z: f32,
    pub position: cgmath::Point3<f32>,
    pub yaw: f32,
    pub pitch: f32
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            zoom_delta: 0.0,
            pan_delta: Vector2::new(0.0, 0.0),
            camera_z: 200.0,
            position: cgmath::Point3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 90.0
        }
    }

    pub fn set_new_position(&mut self, coord: Vector3<f32>) {
        self.position = Point3::new(coord.x, coord.y, 0.0).cast().unwrap()
    }

    pub(crate) fn update_camera(&mut self, camera: &mut Camera) {
        let speed_koef = self.camera_z / 150.0;

        let (sin_pitch, cos_pitch) = Rad::from(Deg(self.pitch)).0.sin_cos();
        let (sin_yaw, cos_yaw) = Rad::from(Deg(-self.yaw)).0.sin_cos();

        let dir = Vector3::new(
            cos_pitch * sin_yaw,
            cos_pitch * cos_yaw,
            sin_pitch,
        ).normalize();

        camera.target = self.position;
        camera.eye += (camera.target - camera.eye).normalize() * self.zoom_delta * speed_koef;

        let len = (camera.target - camera.eye).magnitude();
        camera.eye = camera.target + (dir * len);

        camera.eye += self.pan_delta.extend(0.0) * speed_koef;
        camera.target += self.pan_delta.extend(0.0) * speed_koef;

        let rotation_matrix = Basis3::from_angle_z(Deg(self.yaw));
        camera.up = rotation_matrix.rotate_vector(cgmath::Vector3::unit_y());

        self.position = camera.target.clone();

        self.pan_delta = Vector2::new(0.0, 0.0);
        self.zoom_delta = 0.0;

        self.camera_z = camera.eye.z;
    }
}
