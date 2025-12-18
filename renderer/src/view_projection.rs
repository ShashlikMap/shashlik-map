use cgmath::{Matrix4, SquareMatrix, Transform, Vector2, Vector3, Vector4};
use geo_types::{Coord, coord};

#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f64> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);

#[rustfmt::skip]
const FLIP_Y: Matrix4<f64> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, -1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    0.0, 0.0, 0.0, 1.0,
);

pub(crate) struct ScreenPositionCalculator<'a> {
    matrix: Matrix4<f32>,
    cs_offset: &'a Vector3<f64>,
    config: &'a wgpu::SurfaceConfiguration,
}

impl<'a> ScreenPositionCalculator<'a> {
    pub fn new(matrix: Matrix4<f32>, cs_offset: &'a Vector3<f64>, config: &'a wgpu::SurfaceConfiguration) -> Self {
        Self { matrix, cs_offset, config }
    }
    pub fn screen_position(&self, world_position: Vector3<f64>) -> Coord<f64> {
        let world_position = world_position - self.cs_offset;
        let pos = self.matrix.cast().unwrap() * Vector4::new(world_position.x, world_position.y, 0.0, 1.0);
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
pub(crate) struct ViewProjUniform {
    view_proj: [[f32; 4]; 4],
    inv_screen_size: [f32; 2],
}

pub(crate) struct ViewProjection {
    pub uniform: ViewProjUniform,
    pub cs_offset: Vector3<f64>,
    inv_view_proj_matrix: Matrix4<f64>
}

impl ViewProjection {
    pub fn new() -> Self {
        ViewProjection {
            uniform: ViewProjUniform {
                view_proj: Matrix4::identity().into(),
                inv_screen_size: [0.0, 0.0],
            },
            cs_offset: Vector3::new(0.0, 0.0, 0.0),
            inv_view_proj_matrix: Matrix4::identity()
        }
    }

    pub fn update(&mut self, view_proj_matrix: Matrix4<f64>, cs_offset: Vector3<f64>) {
        self.uniform.view_proj = (FLIP_Y * OPENGL_TO_WGPU_MATRIX * view_proj_matrix)
            .cast()
            .unwrap()
            .into();
        self.cs_offset = cs_offset;
        self.inv_view_proj_matrix = view_proj_matrix.inverse_transform().unwrap();
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.uniform.inv_screen_size = [1.0 / width as f32, 1.0 / height as f32];
    }

    pub fn screen_position_calculator<'a>(
        &self,
        cs_offset: &'a Vector3<f64>,
        config: &'a wgpu::SurfaceConfiguration,
    ) -> ScreenPositionCalculator<'a> {
        ScreenPositionCalculator::new(self.uniform.view_proj.into(), cs_offset, config)
    }

    pub fn clip_to_world(&self, coord: &Coord<f64>) -> Option<Vector2<f64>> {
        Self::clip_to_world_at_ground(
            &Vector2::new(coord.x, coord.y),
            &self.inv_view_proj_matrix.cast().unwrap(),
        ).map(|coord| {
            coord + self.cs_offset.truncate()
        })
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
