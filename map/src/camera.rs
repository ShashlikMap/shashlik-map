use cgmath::{
    Basis3, Deg, Matrix4, Point3, Rotation, Rotation3, SquareMatrix, Vector3
};

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
    pub pan_delta: (f32, f32),
    pub camera_z: f32,
    pub position: cgmath::Point3<f32>,
    pub rotation: f32,
    pub tilt: f32
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            zoom_delta: 0.0,
            pan_delta: (0.0, 0.0),
            camera_z: 200.0,
            position: cgmath::Point3::new(0.0, 0.0, 0.0),
            rotation: 0.0,
            tilt: 0.0
        }
    }

    pub fn set_new_position(&mut self, coord: Vector3<f32>) {
        self.position = Point3::new(coord.x, coord.y, 0.0).cast().unwrap()
    }

    pub(crate) fn update_camera(&mut self, camera: &mut Camera) {
        use cgmath::InnerSpace;
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();

        let speed_koef = self.camera_z / 150.0;

        camera.eye += forward_norm * self.zoom_delta * speed_koef;

        camera.eye.x = self.position.x;
        camera.eye.y = self.position.y;
        camera.target.x = self.position.x;
        camera.target.y = self.position.y;

        camera.eye.x += self.pan_delta.0 * speed_koef;
        camera.target.x += self.pan_delta.0 * speed_koef;
        camera.eye.y += self.pan_delta.1 * speed_koef;
        camera.target.y += self.pan_delta.1 * speed_koef;

        let rotation_matrix = Basis3::from_angle_z(Deg(self.rotation));
        // temporary fast trick for top-down view
        camera.up = rotation_matrix.rotate_vector(cgmath::Vector3::unit_y());

        self.position = camera.target.clone();

        self.pan_delta = (0.0, 0.0);
        self.zoom_delta = 0.0;

        self.camera_z = camera.eye.z;
    }
}
