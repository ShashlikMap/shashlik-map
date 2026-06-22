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
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some(format!("{:?} Buffer", usage).as_str()),
            contents: bytemuck::cast_slice(data),
            usage,
        });
        buffer
    }
    pub fn recycle(&mut self, key: &str) {}
}
