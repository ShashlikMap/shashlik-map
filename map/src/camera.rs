use glam::DMat2;
use glam::DMat3;
use glam::DMat4;
use glam::DVec2;
use glam::DVec3;
use glam::Vec3Swizzles;
use std::f64::consts::PI;
use wgpu_canvas::LIGHT_POS;

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
    pub(crate) const Z_NEAR: f64 = 1.0;
    pub(crate) const Z_FAR: f64 = 8000000.0;
    const LIGHT_DISTANCE: f64 = 100.0;
    const DEFAULT_FOV: f64 = 37.87;

    pub fn new(initial_world: DVec2) -> Self {
        Camera {
            eye:  initial_world.extend(Self::INITIAL_Z * 2.0),
            target: initial_world.extend(0.0),
            up: DVec3::Y,
            fovy: Self::DEFAULT_FOV,
            znear: Self::Z_NEAR,
            zfar: Self::Z_FAR,
            perspective_matrix: DMat4::IDENTITY,
            offset: initial_world.extend(0.0)
        }
    }

    pub fn global_offset(&mut self, world_offset: DVec2) {
        let world_offset = world_offset.extend(0.0);
        self.eye += world_offset;
        self.target += world_offset;
        self.offset += world_offset;
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
            target_offset + Self::LIGHT_DISTANCE * LIGHT_POS,
            target_offset,
            DVec3::Z,
        );
        light_view
    }
    
    pub fn scale(&self) -> f32 {
        (self.eye_direction().length() / Self::INITIAL_Z) as f32
    }

    pub fn eye_direction(&self) -> DVec3 {
        self.eye - self.target
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let aspect = width as f64 / height as f64;
        let mut fovy = self.fovy.to_radians();
        if aspect > 1.0 {
            fovy = 2.0 * ((fovy / 2.0).tan() / aspect).atan();
        }
        self.perspective_matrix =
            DMat4::perspective_rh(
                fovy,
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
    pub const MIN_PITCH: f64 = 55.0;
    pub const MAX_PITCH: f64 = 90.0;

    const ORIGIN_REBASE_THRESHOLD: f64 = 999.0; // random now, big enough between US/JAPAN

    pub fn new() -> Self {
        Self {
            zoom_delta: 0.0,
            pan_delta: DVec2::splat(0.0),
            camera_z: 200.0,
            forward_len: 200.0,
            position: DVec3::splat(0.0),
            yaw: 0.0,
            pitch: Self::MAX_PITCH,
        }
    }

    pub fn set_new_position(&mut self, position: DVec3) {
        self.position = position;
    }

    pub(crate) fn update_camera(&mut self, camera: &mut Camera) {
        // prevent sharp pitch for a high zoom level to reduce z_far artifacts
        let min_pitch = Self::MIN_PITCH + (Self::MAX_PITCH - Self::MIN_PITCH) * (self.camera_z * 10.0 / Camera::Z_FAR).clamp(0.0, 1.0);
        let (sin_pitch, cos_pitch) = self.pitch.max(min_pitch).to_radians().sin_cos();
        let (sin_yaw, cos_yaw) = (-self.yaw).to_radians().sin_cos();

        let dir = DVec3::new(cos_pitch * sin_yaw, cos_pitch * cos_yaw, sin_pitch).normalize();
        let new_eye = if self.zoom_delta == 0.0 {
            camera.eye
        } else {
            camera.target + (dir * ((camera.target - camera.eye).length() * (1.0 / self.zoom_delta)))
        };
        let len = (camera.target - new_eye).length();

        camera.target = self.position;

        let new_eye = camera.target + (dir * len);
        // don't go too far to reduce z_far artifacts, or too close
        let new_eye_target_dist = (camera.target - new_eye).length();
        if new_eye_target_dist <= 0.9 * Camera::Z_FAR && new_eye_target_dist >= 10.0 * Camera::Z_NEAR {
            camera.eye = new_eye;
        }

        let pan_vec = (DMat2::from_angle(self.yaw.to_radians() - PI) * self.pan_delta).extend(0.0);
        camera.eye -= pan_vec;
        camera.target -= pan_vec;

        let distance_from_origin = camera.offset.xy().distance(camera.target.xy());
        if distance_from_origin >= Self::ORIGIN_REBASE_THRESHOLD {
            camera.offset = camera.target.xy().extend(0.0);
        }

        let rotation_matrix = DMat3::from_rotation_z(self.yaw.to_radians());
        camera.up = rotation_matrix * DVec3::Y;

        self.pan_delta = DVec2::splat(0.0);
        self.zoom_delta = 0.0;

        self.forward_len = len;
        self.camera_z = camera.eye.z;
        self.position = camera.target;
    }
}
