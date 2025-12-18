use cgmath::{
    point3, Basis3, Deg, InnerSpace, Matrix4, Rad, Rotation, Rotation3, SquareMatrix,
    Vector2, Vector3,
};

pub struct Camera {
    pub eye: cgmath::Vector3<f64>,
    pub target: cgmath::Vector3<f64>,
    up: cgmath::Vector3<f64>,
    fovy: f64,
    znear: f64,
    zfar: f64,
    perspective_matrix: Matrix4<f64>,
    pub offset: cgmath::Vector3<f64>,
}

impl Camera {
    pub fn new(initial_world: Vector3<f64>) -> Self {
        Camera {
            eye: (initial_world.x, initial_world.y, 200.0).into(),
            target: (initial_world.x, initial_world.y, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            fovy: 45.0,
            znear: 1.0,
            zfar: 2000000.0,
            perspective_matrix: Matrix4::identity(),
            offset: Vector3::new(initial_world.x, initial_world.y, 0.0),
        }
    }

    pub fn build_view_projection_matrix(&mut self) -> cgmath::Matrix4<f64> {
        let eye_offset = self.eye - self.offset;
        let target_offset = self.target - self.offset;
        let view = cgmath::Matrix4::look_at_rh(
            point3(eye_offset.x, eye_offset.y, eye_offset.z),
            point3(target_offset.x, target_offset.y, target_offset.z),
            self.up,
        );
        self.perspective_matrix * view
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let aspect = width as f64 / height as f64;
        self.perspective_matrix =
            cgmath::perspective(cgmath::Deg(self.fovy), aspect, self.znear, self.zfar);
    }
}

pub struct CameraController {
    pub zoom_delta: f64,
    pub pan_delta: Vector2<f64>,
    pub camera_z: f64,
    pub forward_len: f64,
    pub position: cgmath::Vector3<f64>,
    pub yaw: f64,
    pub pitch: f64,
}

impl CameraController {
    const ORIGIN_REBASE_THRESHOLD: f64 = 99999.0; // random now, big enough between US/JAPAN

    pub fn new() -> Self {
        Self {
            zoom_delta: 0.0,
            pan_delta: Vector2::new(0.0, 0.0),
            camera_z: 200.0,
            forward_len: 200.0,
            position: cgmath::Vector3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 90.0,
        }
    }

    pub fn set_new_position(&mut self, position: Vector3<f64>) {
        self.position = position;
    }

    pub(crate) fn update_camera(&mut self, camera: &mut Camera) {
        let speed_koef = self.camera_z / 150.0;

        let (sin_pitch, cos_pitch) = Rad::from(Deg(self.pitch)).0.sin_cos();
        let (sin_yaw, cos_yaw) = Rad::from(Deg(-self.yaw)).0.sin_cos();

        let dir = Vector3::new(cos_pitch * sin_yaw, cos_pitch * cos_yaw, sin_pitch).normalize();

        camera.eye += (camera.target - camera.eye).normalize() * self.zoom_delta * speed_koef;
        let len = (camera.target - camera.eye).magnitude();

        camera.target = self.position;
        camera.eye = camera.target + (dir * len);

        camera.eye += self.pan_delta.extend(0.0) * speed_koef;
        camera.target += self.pan_delta.extend(0.0) * speed_koef;

        let distance_from_origin = (camera.offset
            - Vector3::new(camera.target.x, camera.target.y, camera.target.z))
        .magnitude();
        if distance_from_origin >= Self::ORIGIN_REBASE_THRESHOLD {
            println!("REBASE!!!!!");
            camera.offset = Vector3::new(camera.target.x, camera.target.y, camera.target.z);
        }

        let rotation_matrix = Basis3::from_angle_z(Deg(self.yaw));
        camera.up = rotation_matrix.rotate_vector(cgmath::Vector3::unit_y());

        self.pan_delta = Vector2::new(0.0, 0.0);
        self.zoom_delta = 0.0;

        self.forward_len = len;
        self.camera_z = camera.eye.z;
        self.position = camera.target;
    }
}
