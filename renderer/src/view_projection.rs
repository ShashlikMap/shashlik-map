use cgmath::{Matrix4, SquareMatrix, Transform, Vector2, Vector3, Vector4};
use geo_types::{Coord, coord};
use wgpu::{Buffer, Device, Queue, SurfaceConfiguration};

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

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ViewProjUniform {
    view_proj: [[f32; 4]; 4],
    inv_screen_size: [f32; 2],
}

#[derive(Clone)]
pub struct ViewProjection {
    uniform: ViewProjUniform,
    pub cs_offset: Vector3<f64>,
    pub screen_size: (f64, f64),
    inv_view_proj_matrix: Matrix4<f64>,
    pub uniform_buffer: Buffer
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

        ViewProjection {
            uniform: ViewProjUniform {
                view_proj: Matrix4::identity().into(),
                inv_screen_size: [0.0, 0.0],
            },
            screen_size: (0.0, 0.0),
            cs_offset: Vector3::new(0.0, 0.0, 0.0),
            inv_view_proj_matrix: Matrix4::identity(),
            uniform_buffer,
        }
    }

    pub fn update(&mut self, queue: &Queue, config: &SurfaceConfiguration, view_proj_matrix: Matrix4<f64>, cs_offset: Vector3<f64>) {
        self.uniform.view_proj = (FLIP_Y * OPENGL_TO_WGPU_MATRIX * view_proj_matrix)
            .cast()
            .unwrap()
            .into();
        self.cs_offset = cs_offset;
        self.inv_view_proj_matrix = view_proj_matrix.inverse_transform().unwrap();
        self.screen_size = (config.width as f64, config.height as f64);

        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniform]),
        );
    }

    pub fn screen_position(&self, world_position: &Vector3<f64>) -> Coord<f64> {
        let matrix: Matrix4<f32> = self.uniform.view_proj.into();
        let world_position = world_position - self.cs_offset;
        let pos = matrix.cast().unwrap() * Vector4::new(world_position.x, world_position.y, 0.0, 1.0);
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
