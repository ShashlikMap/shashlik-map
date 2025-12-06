use cgmath::{Matrix4, SquareMatrix, Vector3, Vector4};
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
    config: &'a wgpu::SurfaceConfiguration,
}

impl<'a> ScreenPositionCalculator<'a> {
    pub fn new(matrix: Matrix4<f32>, config: &'a wgpu::SurfaceConfiguration) -> Self {
        Self { matrix, config }
    }
    pub fn screen_position(&self, world_position: Vector3<f64>) -> Coord<f64> {
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

impl ViewProjUniform {
    pub fn new() -> Self {
        ViewProjUniform {
            view_proj: Matrix4::identity().into(),
            inv_screen_size: [0.0, 0.0],
        }
    }

    pub fn update(&mut self, view_proj_matrix: Matrix4<f64>) {
        self.view_proj = (FLIP_Y * OPENGL_TO_WGPU_MATRIX * view_proj_matrix)
            .cast()
            .unwrap()
            .into();
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.inv_screen_size = [1.0 / width as f32, 1.0 / height as f32];
    }

    pub fn screen_position_calculator<'a>(
        &self,
        config: &'a wgpu::SurfaceConfiguration,
    ) -> ScreenPositionCalculator<'a> {
        ScreenPositionCalculator::new(self.view_proj.into(), config)
    }
}
