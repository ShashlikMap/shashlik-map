use crate::{RendererUpdateData, LIGHT_POS};
use geo_types::{coord, Coord};
use glam::{DMat2, DMat4, DVec2, DVec3, DVec4, Mat4, Vec3Swizzles};
use wgpu::{Buffer, Device, Queue, SurfaceConfiguration};

#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: DMat4 = DMat4::from_cols(
    DVec4::new(1.0, 0.0, 0.0, 0.0),
    DVec4::new(0.0, 1.0, 0.0, 0.0),
    DVec4::new(0.0, 0.0, 0.5, 0.0),
    DVec4::new(0.0, 0.0, 0.5, 1.0),
);

#[rustfmt::skip]
const FLIP_Y: DMat4 = DMat4::from_cols_array(
    &[1.0, 0.0, 0.0, 0.0,
    0.0, -1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    0.0, 0.0, 0.0, 1.0],
);

macro_rules! min_f64 {
    ($x:expr) => ($x);
    ($x:expr, $($y:expr),+) => {
        ($x).min(min_f64!($($y),+))
    };
}

macro_rules! max_f64 {
    ($x:expr) => ($x);
    ($x:expr, $($y:expr),+) => {
        ($x).max(max_f64!($($y),+))
    };
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ViewProjUniform {
    view: [[f32; 4]; 4],
    proj: [[f32; 4]; 4],
    view_proj: [[f32; 4]; 4],
    view_proj_inv: [[f32; 4]; 4],
    light_view_proj: [[f32; 4]; 4],
    view_tr_inv: [[f32; 4]; 4],
    inv_screen_size: [f32; 2],
    scale: f32,
    p2_scale: f32
}

#[derive(Clone)]
pub struct ViewProjection {
    uniform: ViewProjUniform,
    pub cs_offset: DVec3,
    pub screen_size: (f64, f64),
    inv_view_proj_matrix: DMat4,
    pub uniform_buffer: Buffer,
    ortho: DMat4
}

impl ViewProjection {
    pub fn new(device: &Device) -> Self {
        // ViewProjection align is 16byte since vec4 is used
        let vec4size = size_of::<[f32; 4]>() as u64;
        let size = size_of::<ViewProjUniform>() as u64;
        let align_mask = vec4size - 1;
        let size = ((size + align_mask) & !align_mask).max(vec4size);
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ViewProjection Buffer"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let ortho = DMat4::orthographic_rh(
            -200.0, 200.0, -200.0, 200.0,
            0.01, 250.0);

        ViewProjection {
            uniform: ViewProjUniform {
                view: Mat4::IDENTITY.to_cols_array_2d(),
                proj: Mat4::IDENTITY.to_cols_array_2d(),
                view_proj: Mat4::IDENTITY.to_cols_array_2d(),
                view_proj_inv: Mat4::IDENTITY.to_cols_array_2d(),
                light_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
                view_tr_inv: Mat4::IDENTITY.to_cols_array_2d(),
                inv_screen_size: [0.0, 0.0],
                scale: 0.0,
                p2_scale: 1.0
            },
            screen_size: (0.0, 0.0),
            cs_offset: DVec3::new(0.0, 0.0, 0.0),
            inv_view_proj_matrix: DMat4::IDENTITY,
            uniform_buffer,
            ortho
        }
    }

    pub fn update(&mut self, queue: &Queue,
                  config: &SurfaceConfiguration,
                  data: RendererUpdateData) {

        self.uniform.view = data.view_matrix.as_mat4()
            .to_cols_array_2d();
        self.uniform.proj = (FLIP_Y * OPENGL_TO_WGPU_MATRIX * data.proj_matrix)
            .as_mat4()
            .to_cols_array_2d();
        let view_proj = FLIP_Y * OPENGL_TO_WGPU_MATRIX * data.view_proj_matrix;

        self.ortho_for_shadow_map(&data);

        self.uniform.light_view_proj = (OPENGL_TO_WGPU_MATRIX * (self.ortho * data.view_light_matrix))
            .as_mat4()
            .to_cols_array_2d();

        self.uniform.view_proj = view_proj
            .as_mat4()
            .to_cols_array_2d();
        let view_proj_inv = view_proj.inverse();
        // No need OPENGL_TO_WGPU_MATRIX?!
        self.uniform.view_proj_inv = (view_proj_inv * FLIP_Y)
            .as_mat4()
            .to_cols_array_2d();

        let view_tr_inv:DMat4 = data.view_matrix.inverse().transpose();
        self.uniform.view_tr_inv = view_tr_inv
            .as_mat4()
            .to_cols_array_2d();
        self.uniform.scale = data.scale;
        self.uniform.p2_scale = self.p2_scale(data.scale);
        self.cs_offset = data.cs_offset;
        self.inv_view_proj_matrix = data.view_proj_matrix.inverse();
        self.screen_size = (config.width as f64, config.height as f64);

        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniform]),
        );
    }

    /// calculate ortho matrix for shadow mapping
    fn ortho_for_shadow_map(&mut self, data: &RendererUpdateData) {
        let c1 = self.clip_to_world(&coord! {x: -1.0, y: -1.0});
        let c2 = self.clip_to_world(&coord! {x: 1.0, y: -1.0});
        let c3 = self.clip_to_world(&coord! {x: -1.0, y: 1.0});
        let c4 = self.clip_to_world(&coord! {x: 1.0, y: 1.0});
        let center = self.clip_to_world(&coord! {x: 0.0, y: 0.0});
        if let (Some(c1), Some(c2), Some(c3), Some(c4), Some(center)) = (c1, c2, c3, c4, center) {
            let light_pos = LIGHT_POS.normalize();
            let mut rad_to_light = data.eye_direction.xy().angle_to(light_pos.xy());
            if rad_to_light.is_nan() {
                rad_to_light = -data.up.xy().angle_to(light_pos.xy());
            }
            let rotation = DMat2::from_angle(rad_to_light);

            let p1 = rotation * (c1 - center);
            let p2 = rotation * (c2 - center);
            let p3 = rotation * (c3 - center);
            let p4 = rotation * (c4 - center);

            let min_x = min_f64!(p1.x, p2.x, p3.x, p4.x);
            let min_y = min_f64!(p1.y, p2.y, p3.y, p4.y);

            let max_x = max_f64!(p1.x, p2.x, p3.x, p4.x);
            let max_y = max_f64!(p1.y, p2.y, p3.y, p4.y) * (1.0 / light_pos.z);

            self.ortho = DMat4::orthographic_rh(
                min_x, max_x, min_y, max_y,
                0.01, 1000.0);
        }
    }

    fn p2_scale(&mut self, scale: f32) -> f32 {
        let p2 = scale.log2().ceil() as u32;
        let mut p2_scale = 1u32;
        if p2 >= 1 {
            p2_scale = 2 << (p2 - 1);
        }
        p2_scale as f32
    }

pub fn screen_position(&self, world_position: &DVec3) -> Coord<f64> {
        let matrix: Mat4 = Mat4::from_cols_array_2d(&self.uniform.view_proj);
        let world_position = world_position - self.cs_offset;
        let pos = matrix.as_dmat4() * DVec4::new(world_position.x, world_position.y, 0.0, 1.0);
        let clip_pos_x = pos.x / pos.w;
        let clip_pos_y = pos.y / pos.w;

        coord! {
            x: self.screen_size.0 * (clip_pos_x + 1.0) / 2.0,
            y: self.screen_size.1 - (self.screen_size.1 * (clip_pos_y + 1.0) / 2.0)
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        // early update of the screen size, otherwise it will come with config but later
        // it may cause incorrect texture sizes and so on

        self.screen_size = (width as f64, height as f64);
        self.uniform.inv_screen_size = [1.0 / width as f32, 1.0 / height as f32];
    }

    pub fn clip_to_world(&self, coord: &Coord<f64>) -> Option<DVec2> {
        Self::clip_to_world_at_ground(
            &DVec2::new(coord.x, coord.y),
            &self.inv_view_proj_matrix,
        ).map(|coord| {
            coord + self.cs_offset.truncate()
        })
    }

    fn clip_to_world_at_ground(
        clip_coords: &DVec2,
        inverted_view_proj: &DMat4,
    ) -> Option<DVec2> {
        let near_world = Self::clip_to_world_internal(
            &DVec3::new(clip_coords.x, clip_coords.y, 0.0),
            inverted_view_proj,
        );

        let far_world = Self::clip_to_world_internal(
            &DVec3::new(clip_coords.x, clip_coords.y, 1.0),
            inverted_view_proj,
        );

        let mut u = -near_world.z / (far_world.z - near_world.z);

        // let's use infinity now but in real world we have to limit it somehow
        // if u < 0.0 { return None };
        if u < 0.0 {
            u = 1.0 - u;
        }
        let result = near_world + u * (far_world - near_world);
        Some(DVec2::new(result.x, result.y))
    }

    fn clip_to_world_internal(
        window: &DVec3,
        inverted_view_proj: &DMat4,
    ) -> DVec3 {
        #[rustfmt::skip]
            let fixed_window = DVec4::new(
            window.x,
            window.y,
            window.z,
            1.0
        );

        let ndc = fixed_window;
        let unprojected = inverted_view_proj * ndc;

        DVec3::new(
            unprojected.x / unprojected.w,
            unprojected.y / unprojected.w,
            unprojected.z / unprojected.w,
        )
    }
}
