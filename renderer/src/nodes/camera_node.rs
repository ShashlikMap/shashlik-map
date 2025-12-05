use crate::camera::{Camera, CameraUniform};
use crate::nodes::scene_tree::RenderContext;
use crate::nodes::SceneNode;
use crate::GlobalContext;
use cgmath::{Matrix4, SquareMatrix, Vector2};
use wgpu::{BindGroupLayout, Device, Queue, RenderPass};

pub struct CameraNode {
    camera: Camera,
    uniform: CameraUniform,
    buffer: wgpu::Buffer,
    bind_group_layout: BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl CameraNode {
    pub fn new(
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
    ) -> Self {
        let mut camera = Camera {
            eye: (0.0, 0.0, 200.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            inv_screen_size: Vector2::new(1.0 / config.width as f32, 1.0 / config.height as f32),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 1.0,
            zfar: 2000000.0,
            perspective_matrix: Matrix4::identity(),
            matrix: Matrix4::identity(),
        };
        // FIXME Android should call resize by itself!
        camera.resize();

        let mut uniform = CameraUniform::new();
        uniform.update_view_proj(&mut camera);

        // CameraUniform align is 16byte since vec4 is used
        let vec4size = size_of::<[f32; 4]>() as u64;
        let size = size_of::<CameraUniform>() as u64;
        let align_mask = vec4size - 1;
        let size = ((size + align_mask) & !align_mask).max(vec4size);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Buffer"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        Self {
            camera,
            uniform,
            buffer,
            bind_group_layout,
            bind_group,
        }
    }
}

impl SceneNode for CameraNode {
    fn setup(&mut self, render_context: &mut RenderContext, _device: &Device) {
        render_context
            .bind_group_layouts
            .push(self.bind_group_layout.clone());
    }
    fn update(
        &mut self,
        _device: &Device,
        queue: &Queue,
        _config: &wgpu::SurfaceConfiguration,
        global_context: &mut GlobalContext,
    ) {
        let camera_controller = &global_context.camera_controller;
        camera_controller
            .borrow_mut()
            .update_camera(&mut self.camera);
        self.uniform.update_view_proj(&mut self.camera);
        camera_controller.borrow_mut().cached_matrix = self.camera.matrix;
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }

    fn render(&self, render_pass: &mut RenderPass, _global_context: &mut GlobalContext) {
        render_pass.set_bind_group(0, &self.bind_group, &[]);
    }

    fn resize(&mut self, width: u32, height: u32, _queue: &Queue) {
        if width > 0 && height > 0 {
            self.camera.aspect = width as f32 / height as f32;
            self.camera.inv_screen_size = Vector2::new(1.0/width as f32, 1.0/height as f32);
            self.camera.resize();
        }
    }
}
