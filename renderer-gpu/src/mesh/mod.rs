use crate::global_context::GlobalContext;
use crate::mesh_buffers::BufferWithId;
use bytemuck::Pod;
use std::marker::PhantomData;
use wgpu::util::DeviceExt;

pub mod mesh;
pub mod mesh_instance_input;
pub mod positioned_mesh;
pub(crate) mod virtual_ground_mesh;

pub struct InstanceBuffer<T: Pod> {
    pub buffer_with_id: Option<BufferWithId>,
    pub length: usize,
    max_length: usize,
    _phantom_data: PhantomData<T>,
}

impl<T: Pod> Default for InstanceBuffer<T> {
    fn default() -> Self {
        InstanceBuffer {
            buffer_with_id: None,
            length: 0,
            max_length: 0,
            _phantom_data: Default::default(),
        }
    }
}

impl<T: Pod> InstanceBuffer<T> {
    pub fn update(&mut self, label: &'static str, global_context: &GlobalContext, data: &Vec<T>) {
        // don't recreate the buffer if its max length doesn't grow
        // TODO benchmark create_buffer_init vs write_buffer vs write_buffer_with
        let data_len = data.len();
        if data_len <= self.max_length
            && let Some(buffer_with_id) = self.buffer_with_id.as_ref()
        {
            // write only non-zero data, if zero then only write once
            if data_len != 0 || data_len != self.length {
                let queue = global_context.queue();
                queue.write_buffer(buffer_with_id.buffer(), 0, bytemuck::cast_slice(data.as_slice()));
                // FIXME write_buffer_with doesn't work as expected, why?
                // if let Some(mut buf_view) = queue.write_buffer_with(buffer, 0, data_size) {
                //     buf_view.copy_from_slice(data);
                // }
            }
        } else {
            let device = global_context.device();
            self.buffer_with_id = Some(
                BufferWithId::from(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: bytemuck::cast_slice(data.as_slice()),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                })),
            );
        }
        self.length = data.len();
        self.max_length = self.max_length.max(self.length);
    }
}
