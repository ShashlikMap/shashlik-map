use cgmath::{Matrix4, SquareMatrix, Transform, Vector2, Vector3, Vector4};
use geo_types::Coord;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);

#[rustfmt::skip]
pub const FLIP_Y: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, -1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    0.0, 0.0, 0.0, 1.0,
);

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
    pub matrix: cgmath::Matrix4<f32>,
}

impl Camera {
    pub fn build_view_projection_matrix(&mut self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        self.matrix = proj * view;
        self.matrix
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    ratio: f32
}

impl CameraUniform {
    pub(crate) fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
            ratio: 1.0
        }
    }

    pub(crate) fn update_view_proj(&mut self, camera: &mut Camera) {
        self.view_proj = (FLIP_Y * OPENGL_TO_WGPU_MATRIX * camera.build_view_projection_matrix()).into();
        self.ratio = camera.aspect;
    }
}

pub struct CameraController {
    speed: f32,
    pub is_up_pressed: bool,
    pub is_down_pressed: bool,
    pub is_forward_pressed: bool,
    pub is_backward_pressed: bool,
    pub is_left_pressed: bool,
    pub is_right_pressed: bool,
    pub is_z_pressed: bool,
    pub is_x_pressed: bool,
    pub is_n_pressed: bool,
    pub is_m_pressed: bool,
    pub cached_matrix: Matrix4<f32>,
    pub camera_z: f32,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_up_pressed: false,
            is_down_pressed: false,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_z_pressed: false,
            is_x_pressed: false,
            is_n_pressed: false,
            is_m_pressed: false,
            cached_matrix: Matrix4::identity().into(),
            camera_z: 200.0
        }
    }

    pub(crate) fn update_camera(&mut self, camera: &mut Camera) {
        use cgmath::InnerSpace;
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        // Prevents glitching when camera gets too close to the
        // center of the scene.
        if self.is_forward_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed;
        }

        let _right = forward_norm.cross(camera.up);

        // Redo radius calc in case the up/ down is pressed.
        let forward = camera.target - camera.eye;
        let _forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // Rescale the distance between the target and eye so
            // that it doesn't change. The eye therefore still
            // lies on the circle made by the target and eye.
            camera.eye.x += self.speed;
            camera.target.x += self.speed;
        }
        if self.is_left_pressed {
            camera.eye.x -= self.speed;
            camera.target.x -= self.speed;
        }

        if self.is_up_pressed {
            camera.eye.y -= self.speed;
            camera.target.y -= self.speed;
        }

        if self.is_down_pressed {
            camera.eye.y += self.speed;
            camera.target.y += self.speed;
        }

        if self.is_z_pressed {
            camera.eye.y += self.speed;
        }

        if self.is_x_pressed {
            camera.eye.y -= self.speed;
        }

        if self.is_n_pressed {
            camera.eye.x -= self.speed;
        }

        if self.is_m_pressed {
            camera.eye.x += self.speed;
        }

        self.camera_z = camera.eye.z;
    }

    pub fn clip_to_world(&self, coord: &Coord<f64>) -> Option<Vector2<f64>> {
        let camera_matrix = self.cached_matrix;
        let inv_mat = camera_matrix.inverse_transform().unwrap();
        Self::clip_to_world_at_ground(
            &Vector2::new(coord.x, coord.y),
            &inv_mat.cast().unwrap(),
        )
    }

    fn clip_to_world_at_ground(
        clip_coords: &Vector2<f64>,
        inverted_view_proj: &Matrix4<f64>,
    ) -> Option<Vector2<f64>> {
        let near_world = Self::clip_to_world_internal(
            &Vector3::new(clip_coords.x, clip_coords.y, 0.0),
            inverted_view_proj,
        );

        let far_world = Self::clip_to_world_internal(
            &Vector3::new(clip_coords.x, clip_coords.y, 1.0),
            inverted_view_proj,
        );

        let mut u = -near_world.z / (far_world.z - near_world.z);

        // let's use infinity now but in real world we have to limit it somehow
        // if u < 0.0 { return None };
        if u < 0.0 {
            u = 1.0 - u;
        }
        let result = near_world + u * (far_world - near_world);
        Some(Vector2::new(result.x, result.y))
    }

    fn clip_to_world_internal(window: &Vector3<f64>, inverted_view_proj: &Matrix4<f64>) -> Vector3<f64> {
        #[rustfmt::skip]
            let fixed_window = Vector4::new(
            window.x,
            window.y,
            window.z,
            1.0
        );

        let ndc = fixed_window;
        let unprojected = inverted_view_proj * ndc;

        Vector3::new(
            unprojected.x / unprojected.w,
            unprojected.y / unprojected.w,
            unprojected.z / unprojected.w,
        )
    }
}