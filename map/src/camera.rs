use glam::dvec3;
use glam::DMat2;
use glam::DMat3;
use glam::DMat4;
use glam::DVec2;
use glam::DVec3;
use std::f64::consts::PI;
use renderer::LIGHT_POS;

pub struct Camera {
    pub eye: DVec3,
    pub target: DVec3,
    pub up: DVec3,
    fovy: f64,
    znear: f64,
    zfar: f64,
    pub perspective_matrix: DMat4,
    pub offset: DVec3,
}

impl Camera {
    const INITIAL_Z: f64 = 200.0;
    pub fn new(initial_world: DVec3) -> Self {
        Camera {
            eye: (initial_world.x, initial_world.y, Self::INITIAL_Z).into(),
            target: (initial_world.x, initial_world.y, 0.0).into(),
            up: DVec3::Y,
            fovy: 45.0,
            znear: 1.0,
            zfar: 2000000.0,
            perspective_matrix: DMat4::IDENTITY,
            offset: DVec3::new(initial_world.x, initial_world.y, 0.0),
        }
    }

    /// view + view_proj matrices
    pub fn build_view_projection_matrix(&mut self) -> (DMat4, DMat4) {
        let eye_offset = self.eye - self.offset;
        let target_offset = self.target - self.offset;
        let view = DMat4::look_at_rh(
            eye_offset,
            target_offset,
            self.up,
        );
        (view, self.perspective_matrix * view)
    }

    /// view_light
    pub fn build_view_light_matrix(&mut self) -> DMat4 {
        let target_offset = self.target - self.offset;
        let light_view = DMat4::look_at_rh(
            target_offset + LIGHT_POS,
            target_offset,
            DVec3::Z,
        );
        light_view
    }
    
    pub fn scale(&self) -> f32 {
        (self.eye.z / Self::INITIAL_Z) as f32
    }

    pub fn eye_direction(&self) -> DVec3 {
        self.eye - self.target
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let aspect = width as f64 / height as f64;

        self.perspective_matrix =
            DMat4::perspective_rh(
                self.fovy.to_radians(),
                aspect, self.znear, self.zfar
            )
    }
}

pub struct CameraController {
    pub zoom_delta: f64,
    pub pan_delta: DVec2,
    pub camera_z: f64,
    pub forward_len: f64,
    pub position: DVec3,
    pub yaw: f64,
    pub pitch: f64,
}

impl CameraController {
    const ORIGIN_REBASE_THRESHOLD: f64 = 99999.0; // random now, big enough between US/JAPAN

    pub fn new() -> Self {
        Self {
            zoom_delta: 0.0,
            pan_delta: DVec2::new(0.0, 0.0),
            camera_z: 200.0,
            forward_len: 200.0,
            position: DVec3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 90.0,
        }
    }

    pub fn set_new_position(&mut self, position: DVec3) {
        self.position = position;
    }

    pub(crate) fn update_camera(&mut self, camera: &mut Camera) {
        let speed_koef = self.camera_z / 150.0;

        let (sin_pitch, cos_pitch) = self.pitch.to_radians().sin_cos();
        let (sin_yaw, cos_yaw) = (-self.yaw).to_radians().sin_cos();

        let dir = DVec3::new(cos_pitch * sin_yaw, cos_pitch * cos_yaw, sin_pitch).normalize();

        camera.eye += (camera.target - camera.eye).normalize() * self.zoom_delta * speed_koef;
        let len = (camera.target - camera.eye).length();

        camera.target = self.position;
        camera.eye = camera.target + (dir * len);

        let pan_vec = (DMat2::from_angle(self.yaw.to_radians() - PI) * self.pan_delta).extend(0.0);
        camera.eye -= pan_vec * speed_koef;
        camera.target -= pan_vec * speed_koef;

        let distance_from_origin = (camera.offset
            - DVec3::new(camera.target.x, camera.target.y, camera.target.z))
        .length();
        if distance_from_origin >= Self::ORIGIN_REBASE_THRESHOLD {
            println!("Origin rebase!");
            camera.offset = DVec3::new(camera.target.x, camera.target.y, camera.target.z);
        }

        let rotation_matrix = DMat3::from_rotation_z(self.yaw.to_radians());
        camera.up = rotation_matrix * DVec3::Y;

        self.pan_delta = DVec2::new(0.0, 0.0);
        self.zoom_delta = 0.0;

        self.forward_len = len;
        self.camera_z = camera.eye.z;
        self.position = camera.target;
    }
}
