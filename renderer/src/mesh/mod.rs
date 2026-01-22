use bytemuck::Pod;
use std::marker::PhantomData;
use wgpu::util::DeviceExt;
use wgpu::{Buffer, Device, Queue};

pub mod mesh;
pub mod mesh_instance_input;
pub mod positioned_mesh;

pub struct InstanceBuffer<T: Pod> {
    pub buffer: Option<Buffer>,
    pub length: usize,
    _phantom_data: PhantomData<T>,
}

impl<T: Pod> Default for InstanceBuffer<T> {
    fn default() -> Self {
        InstanceBuffer {
            buffer: None,
            length: 0,
            _phantom_data: Default::default(),
        }
    }
}

impl<T: Pod> InstanceBuffer<T> {
    pub fn update(&mut self, label: &'static str, device: &Device, queue: &Queue, data: &Vec<T>) {
        if data.len() <= self.length
            && let Some(buffer) = self.buffer.as_ref()
        {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(data.as_slice()));
        } else {
            self.buffer = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: bytemuck::cast_slice(data.as_slice()),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                }),
            );
        }
        self.length = data.len();
    }
}
