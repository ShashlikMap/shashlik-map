use bytemuck::NoUninit;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{Buffer, Device, wgt};

pub(crate) struct BufferPool {}

impl BufferPool {
    pub fn create<'a, T: NoUninit>(
        &mut self,
        device: &Device,
        key: Option<&str>,
        usage: wgt::BufferUsages,
        data: &'a [T],
    ) -> Buffer {
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(data),
            usage,
        });
        vertex_buffer
    }
    pub fn recycle(&mut self, key: &str) {}
}
