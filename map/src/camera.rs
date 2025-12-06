use cgmath::{
    Basis3, Deg, Matrix4, Point3, Rotation, Rotation3, Transform, Vector2, Vector3,
    Vector4,
};
use geo_types::Coord;


pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
    pub perspective_matrix: Matrix4<f32>,
    pub inv_view_proj_matrix: Matrix4<f64>,
}

impl Camera {
    pub fn build_view_projection_matrix(&mut self) -> cgmath::Matrix4<f64> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let view_proj_matrix = (self.perspective_matrix * view).cast().unwrap();
        self.inv_view_proj_matrix = view_proj_matrix.inverse_transform().unwrap();
        view_proj_matrix
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let aspect = width as f32 / height as f32;
        self.perspective_matrix =
            cgmath::perspective(cgmath::Deg(self.fovy), aspect, self.znear, self.zfar);
    }

    pub fn clip_to_world(&self, coord: &Coord<f64>) -> Option<Vector2<f64>> {
        Self::clip_to_world_at_ground(&Vector2::new(coord.x, coord.y), &self.inv_view_proj_matrix.cast().unwrap())
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

pub struct CameraController {
    #[allow(dead_code)]
    speed: f32,
    pub zoom_delta: f32,
    pub pan_delta: (f32, f32),
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
            camera_z: 200.0,
            position: cgmath::Point3::new(0.0, 0.0, 0.0),
            rotation: 0.0,
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
