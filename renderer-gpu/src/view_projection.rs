use crate::Renderer;
use crate::render_config::RenderConfig;
use crate::{GpuRenderer, RendererUpdateData};
use geo_types::{Coord, coord};
use glam::{DMat4, DVec2, DVec3, DVec4, Mat4, Vec2, Vec3Swizzles, Vec4Swizzles};
use renderer_common::{max_f64, min_f64, GLOBE_R, GLOBE_SCALE, LIGHT_POS, MAP_SIZE};
use std::cmp::min;
use std::f64::consts::PI;
use wgpu::{Buffer, Device, Queue, SurfaceConfiguration};

#[rustfmt::skip]
const FLIP_Y: DMat4 = DMat4::from_cols_array(
    &[1.0, 0.0, 0.0, 0.0,
    0.0, -1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    0.0, 0.0, 0.0, 1.0],
);

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ViewProjUniform {
    view: [[f32; 4]; 4],
    proj: [[f32; 4]; 4],
    view_proj: [[f32; 4]; 4],
    globe_view_proj: [[f32; 4]; 4],
    view_proj_inv: [[f32; 4]; 4],
    light_view_proj: [[f32; 4]; 4],
    view_tr_inv: [[f32; 4]; 4],
    inv_screen_size: [f32; 2],
    pub(crate) scale: f32,
    p2_scale: f32,
    scale_2d_3d: f32,
    globe_r: f32
}

#[derive(Clone)]
pub(crate) struct ViewProjection {
    pub uniform: ViewProjUniform,
    pub scale_2d_3d: f32,
    cs_offset: DVec3,
    pub screen_size: (f64, f64),
    globe_view: DMat4,
    inv_view_proj_matrix: DMat4,
    inv_globe_view_proj_matrix: DMat4,
    pub uniform_buffer: Buffer,
    ortho: DMat4,
    is_shadow_enabled: bool,
    shadow_texture_size: (u32, u32),
    round_screen_sq_radius: Option<f32>
}

impl ViewProjection {
    const MAX_MESH_HEIGHT: f64 = 40.0;

    const ORTHO_STEP: f64 = 5.0;

    // don't use 0.0 or close to that to reduce amount of points on the globe edge
    const GLOBE_HIDDEN_SIZE_THRESHOLD: f64 = 150000.0;

    pub fn new(device: &Device, render_config: &RenderConfig) -> Self {
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
                globe_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
                view_proj_inv: Mat4::IDENTITY.to_cols_array_2d(),
                light_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
                view_tr_inv: Mat4::IDENTITY.to_cols_array_2d(),
                inv_screen_size: [0.0, 0.0],
                scale: 0.0,
                p2_scale: 1.0,
                scale_2d_3d: 1.0,
                globe_r: 0.0,
            },
            scale_2d_3d: 0.0,
            screen_size: (0.0, 0.0),
            cs_offset: DVec3::new(0.0, 0.0, 0.0),
            globe_view: DMat4::IDENTITY,
            inv_view_proj_matrix: DMat4::IDENTITY,
            inv_globe_view_proj_matrix: DMat4::IDENTITY,
            uniform_buffer,
            ortho,
            is_shadow_enabled: render_config.shadow_enabled,
            shadow_texture_size: render_config.shadow_texture_size(),
            round_screen_sq_radius: render_config.round_screen().then_some(0.0f32),
        }
    }

    pub fn update(&mut self, queue: &Queue,
                  render_config: &RenderConfig,
                  config: &SurfaceConfiguration,
                  mut data: RendererUpdateData) {

        self.uniform.view = data.view_matrix.as_mat4()
            .to_cols_array_2d();
        self.uniform.proj = (FLIP_Y * data.proj_matrix)
            .as_mat4()
            .to_cols_array_2d();
        let view_proj = FLIP_Y * data.view_proj_matrix;
        let globe_view_proj = FLIP_Y * data.globe_view_proj_matrix;

        self.shadow_texture_size = render_config.shadow_texture_size();
        self.is_shadow_enabled = render_config.shadow_enabled;
        self.scale_2d_3d = data.scale_2d_3d;

        self.ortho_for_shadow_map(&mut data.view_light_matrix, data.scale);

        self.uniform.light_view_proj = (self.ortho * data.view_light_matrix)
            .as_mat4()
            .to_cols_array_2d();

        self.uniform.view_proj = view_proj
            .as_mat4()
            .to_cols_array_2d();

        self.globe_view = data.globe_view;
        self.uniform.globe_view_proj = globe_view_proj
            .as_mat4()
            .to_cols_array_2d();
        self.inv_globe_view_proj_matrix = globe_view_proj.inverse();

        let view_proj_inv = view_proj.inverse();
        self.uniform.view_proj_inv = (view_proj_inv * FLIP_Y)
            .as_mat4()
            .to_cols_array_2d();

        let view_tr_inv:DMat4 = data.view_matrix.inverse().transpose();
        self.uniform.view_tr_inv = view_tr_inv
            .as_mat4()
            .to_cols_array_2d();
        self.uniform.scale = data.scale;
        self.uniform.globe_r = data.globe_r;

        self.uniform.p2_scale = self.p2_scale(data.scale);
        self.uniform.scale_2d_3d = data.scale_2d_3d;
        self.cs_offset = data.cs_offset;
        self.inv_view_proj_matrix = data.view_proj_matrix.inverse();
        self.screen_size = (config.width as f64, config.height as f64);

        if let Some(ref mut radius_sq) = self.round_screen_sq_radius {
            *radius_sq = ((min(config.width, config.height) as f32) * 0.5f32).powf(2f32);
        }

        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniform]),
        );
    }

    /// calculate ortho matrix for shadow mapping
    fn ortho_for_shadow_map(&mut self, view_light_matrix: &mut DMat4, scale: f32) {
        if !self.is_shadow_mapping_enabled() {
            return;
        }
        let c1 = self.clip_to_world(&coord! {x: -1.0, y: -1.0});
        let c2 = self.clip_to_world(&coord! {x: 1.0, y: -1.0});
        let c3 = self.clip_to_world(&coord! {x: -1.0, y: 1.0});
        let c4 = self.clip_to_world(&coord! {x: 1.0, y: 1.0});
        if let (Some(c1), Some(c2), Some(c3), Some(c4)) = (c1, c2, c3, c4) {
            let center = ((c1 + c2 + c3 + c4) / 4.0).extend(0.0);
            let light_view = DMat4::look_at_rh(center + LIGHT_POS, center, DVec3::Z);

            let p1 = light_view.transform_point3(c1.extend(0.0));
            let p2 = light_view.transform_point3(c2.extend(0.0));
            let p3 = light_view.transform_point3(c3.extend(0.0));
            let p4 = light_view.transform_point3(c4.extend(0.0));

            // dynamically change bounds, so MAX_MESH_HEIGHT won't affect much the matrix size when close to ground
            let max_mesh_height = (Self::MAX_MESH_HEIGHT * (1.5 * scale as f64)).min(Self::MAX_MESH_HEIGHT);
            let min_x = min_f64!(p1.x, p2.x, p3.x, p4.x) - max_mesh_height;
            let min_y = min_f64!(p1.y, p2.y, p3.y, p4.y) - max_mesh_height;
            let max_x = max_f64!(p1.x, p2.x, p3.x, p4.x) + max_mesh_height;
            let max_y = max_f64!(p1.y, p2.y, p3.y, p4.y) + max_mesh_height;

            let depth_texture_size = self.shadow_texture_size.0 as f64;
            let ortho_width = ((max_x - min_x) / Self::ORTHO_STEP).round() * Self::ORTHO_STEP;
            let ortho_height = ((max_y - min_y) / Self::ORTHO_STEP).round() * Self::ORTHO_STEP;

            let texel_size_x = ortho_width / depth_texture_size;
            let texel_size_y = ortho_height / depth_texture_size;

            let mut light_space_translation = view_light_matrix.w_axis.xyz();
            light_space_translation.x = (light_space_translation.x / texel_size_x).floor() * texel_size_x;
            light_space_translation.y = (light_space_translation.y / texel_size_y).floor() * texel_size_y;
            view_light_matrix.w_axis = light_space_translation.extend(1.0);

            self.ortho = DMat4::orthographic_rh(
                -ortho_width / 2.0, ortho_width / 2.0, -ortho_height / 2.0, ortho_height / 2.0,
                0.01, 1600.0);
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

    fn transform_to_globe_position(position: DVec2) -> DVec3 {
        let merc = position / MAP_SIZE;
        let lat = 2.0 * (PI * (1.0 - 2.0 * merc.y)).exp().atan() - PI * 0.5;
        let lon = 2.0 * PI * (merc.x - 0.5);
        DVec3::new(lat.cos() * lon.sin(), lat.cos() * lon.cos(), lat.sin())
    }

    /// Coordinate on the screen + visibility on the globe.
    /// For flat screen it's always true
    pub fn screen_position(&self, world_position: &DVec3) -> (Coord<f64>, bool) {
        let matrix: Mat4 = if self.is_globe_view() {
            Mat4::from_cols_array_2d(&self.uniform.globe_view_proj)
        } else {
            Mat4::from_cols_array_2d(&self.uniform.view_proj)
        };
        let world_position = world_position - self.get_cs_offset();
        let mut visible_on_globe = true;
        let world_position = if self.is_globe_view() {
            let ret = Self::transform_to_globe_position(world_position.xy()) * GLOBE_R;
            let relative_to_target = self.globe_view.transform_vector3(ret);
            if relative_to_target.z <= Self::GLOBE_HIDDEN_SIZE_THRESHOLD {
                visible_on_globe = false
            }
            ret.extend(1.0)
        } else {
            DVec4::new(world_position.x, world_position.y, 0.0, 1.0)
        };

        let pos = matrix.as_dmat4() * world_position;
        let clip_pos_x = pos.x / pos.w;
        let clip_pos_y = pos.y / pos.w;

        (coord! {
            x: self.screen_size.0 * (clip_pos_x + 1.0) / 2.0,
            y: self.screen_size.1 - (self.screen_size.1 * (clip_pos_y + 1.0) / 2.0),
        }, visible_on_globe)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        // early update of the screen size, otherwise it will come with config but later
        // it may cause incorrect texture sizes and so on

        self.screen_size = (width as f64, height as f64);
        self.uniform.inv_screen_size = [1.0 / width as f32, 1.0 / height as f32];
    }

    pub fn screen_to_world(&self, coord: &Vec2) -> Option<DVec2> {
        self.clip_to_world(&coord! { x : (coord.x as f64 / self.screen_size.0) * 2.0 - 1.0,
            y : (coord.y as f64 / self.screen_size.1) * 2.0 - 1.0})
    }

    pub fn clip_to_world(&self, coord: &Coord<f64>) -> Option<DVec2> {
        if self.is_globe_view() {
            <GpuRenderer as Renderer>::clip_to_world_at_globe(
                &DVec2::new(coord.x, coord.y),
                &self.inv_globe_view_proj_matrix,
            ).map(|coord| {
                // basically, it's opposite of transform_to_globe_position
                // It's needed because this conversion happens in shaders
                let n = coord.normalize();
                let lat = n.z.asin();
                let lon = n.x.atan2(n.y);
                let merc_x = (lon / (2.0 * PI)) + 0.5;
                let merc_y = 0.5 - (((lat + PI * 0.5) * 0.5).tan().ln() / (2.0 * PI));
                DVec2::new(merc_x, merc_y) * MAP_SIZE
            }).or_else(|| {
                // if now result then the ray misses planet, return the closest result to map
                Some(MAP_SIZE * (DVec2::new(coord.x, coord.y) + 1.0) * 0.5)
            })
        } else {
            <GpuRenderer as Renderer>::clip_to_world_at_ground(
                &DVec2::new(coord.x, coord.y),
                &self.inv_view_proj_matrix,
            ).map(|coord| {
                coord.truncate() + self.cs_offset.truncate()
            })
        }
    }

    pub fn is_shadow_mapping_enabled(&self) -> bool {
        (self.scale_2d_3d > 0.0) && self.is_shadow_enabled
    }

    pub fn round_screen_sq_radius(&self) -> Option<f32> {
        self.round_screen_sq_radius
    }

    pub fn is_globe_view(&self) -> bool {
        self.uniform.scale > GLOBE_SCALE
    }

    pub fn get_cs_offset(&self) -> DVec3 {
       if self.is_globe_view() {
            DVec3::splat(0.0)
        } else {
            self.cs_offset.clone()
        }
    }
}
