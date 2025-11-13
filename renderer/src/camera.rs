use cgmath::{
    Basis3, Deg, Matrix4, Point3, Rotation, Rotation3, SquareMatrix, Transform, Vector2,
    Vector3, Vector4,
};
use geo_types::{coord, Coord};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f64> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);

#[rustfmt::skip]
pub const FLIP_Y: Matrix4<f64> = Matrix4::new(
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
    pub matrix: cgmath::Matrix4<f64>,
}

impl Camera {
    pub fn build_view_projection_matrix(&mut self) -> cgmath::Matrix4<f64> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        self.matrix = (proj * view).cast().unwrap();
        self.matrix
    }
}

pub struct ScreenPositionCalculator<'a> {
    matrix: Matrix4<f64>,
    config: &'a wgpu::SurfaceConfiguration,
}

impl<'a> ScreenPositionCalculator<'a> {
    pub fn new(matrix: Matrix4<f64>, config: &'a wgpu::SurfaceConfiguration) -> Self {
        Self { matrix, config }
    }
    pub fn screen_position(&self, world_position: Vector3<f64>) -> Coord<f64> {
        let pos = self.matrix * Vector4::new(world_position.x, world_position.y, 0.0, 1.0);
        let clip_pos_x = pos.x / pos.w;
        let clip_pos_y = pos.y / pos.w;

        let screen_size = (self.config.width as f64, self.config.height as f64);
        coord! {
            x: screen_size.0 * (clip_pos_x + 1.0) / 2.0,
            y: screen_size.1 - (screen_size.1 * (clip_pos_y + 1.0) / 2.0)
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    ratio: f32,
}

impl CameraUniform {
    pub(crate) fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
            ratio: 1.0,
        }
    }

    pub(crate) fn update_view_proj(&mut self, camera: &mut Camera) {
        self.view_proj = (FLIP_Y * OPENGL_TO_WGPU_MATRIX * camera.build_view_projection_matrix())
            .cast()
            .unwrap()
            .into();
        self.ratio = camera.aspect;
    }
}

pub struct CameraController {
    #[allow(dead_code)]
    speed: f32,
    pub zoom_delta: f32,
    pub pan_delta: (f32, f32),
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
    pub cached_matrix: Matrix4<f64>,
    pub camera_z: f32,
    pub position: cgmath::Point3<f32>,
    pub rotation: f32,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            zoom_delta: 0.0,
            pan_delta: (0.0, 0.0),
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
            camera_z: 200.0,
            position: cgmath::Point3::new(0.0, 0.0, 0.0),
            rotation: 0.0,
        }
    }

    pub fn screen_position_calculator<'a>(
        &self,
        config: &'a wgpu::SurfaceConfiguration,
    ) -> ScreenPositionCalculator<'a> {
        let matrix = FLIP_Y * OPENGL_TO_WGPU_MATRIX * self.cached_matrix;
        ScreenPositionCalculator::new(matrix, config)
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

    pub fn clip_to_world(&self, coord: &Coord<f64>) -> Option<Vector2<f64>> {
        let camera_matrix = self.cached_matrix;
        let inv_mat = camera_matrix.inverse_transform().unwrap();
        Self::clip_to_world_at_ground(&Vector2::new(coord.x, coord.y), &inv_mat.cast().unwrap())
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

    fn clip_to_world_internal(
        window: &Vector3<f64>,
        inverted_view_proj: &Matrix4<f64>,
    ) -> Vector3<f64> {
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
