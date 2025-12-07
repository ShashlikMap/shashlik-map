use crate::nodes::scene_tree::RenderContext;
use crate::nodes::SceneNode;
use crate::view_projection::ViewProjUniform;
use crate::GlobalContext;
use wgpu::{BindGroupLayout, Device, Queue, RenderPass};

pub struct CameraNode {
    buffer: wgpu::Buffer,
    bind_group_layout: BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl CameraNode {
    pub fn new(device: &wgpu::Device) -> Self {
        // CameraUniform align is 16byte since vec4 is used
        let vec4size = size_of::<[f32; 4]>() as u64;
        let size = size_of::<ViewProjUniform>() as u64;
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
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(&[global_context.view_projection.uniform]),
        );
    }

    fn render(&self, render_pass: &mut RenderPass, _global_context: &mut GlobalContext) {
        render_pass.set_bind_group(0, &self.bind_group, &[]);
    }
}
