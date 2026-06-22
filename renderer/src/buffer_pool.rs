use bytemuck::NoUninit;
use std::collections::HashMap;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{Buffer, BufferAddress, COPY_BUFFER_ALIGNMENT, Device, Queue, wgt, BufferUsages};

pub(crate) struct BufferPool {
    recycled: Vec<Buffer>,
    used: HashMap<String, Vec<Buffer>>,
}

impl BufferPool {
    pub fn new() -> Self {
        Self {
            recycled: Vec::new(),
            used: HashMap::default(),
        }
    }
    pub fn create<'a, T: NoUninit>(
        &mut self,
        device: &Device,
        queue: &Queue,
        key: Option<&str>,
        usage: wgt::BufferUsages,
        data: &'a [T],
    ) -> Buffer {
        let data = bytemuck::cast_slice(data);

        if key.is_none() {
            return device.create_buffer_init(&BufferInitDescriptor {
                label: Some(format!("{:?} Buffer", usage).as_str()),
                contents: data,
                usage,
            });
        }

        self.recycled
            .sort_by(|item1, item2| item1.size().cmp(&item2.size()));

        let actual_usage = usage | BufferUsages::COPY_DST;
        let padded_size = self.padded_size(data);
        let mut new_buffer = self
            .recycled
            .iter()
            .position(|item| item.size() >= padded_size && item.usage() == actual_usage)
            .map(|index| self.recycled.remove(index));

        let buffer = match new_buffer {
            Some(value) => {
                queue.write_buffer(&value, 0, data);
                value
            }
            None => new_buffer
                .insert(device.create_buffer_init(&BufferInitDescriptor {
                    label: Some(format!("{:?} Buffer", usage).as_str()),
                    contents: data,
                    usage: actual_usage,
                }))
                .clone(),
        };

        self.used
            .entry(key.unwrap().to_string())
            .or_default()
            .push(buffer.clone());

        buffer
    }
    pub fn recycle(&mut self, key: &str) {
        self.recycled
            .extend(self.used.remove(key).into_iter().flatten());
    }

    fn padded_size(&self, data: &[u8]) -> BufferAddress {
        let unpadded_size = data.len() as BufferAddress;
        let align_mask = COPY_BUFFER_ALIGNMENT - 1;
        let padded_size = ((unpadded_size + align_mask) & !align_mask).max(COPY_BUFFER_ALIGNMENT);
        padded_size
    }
}
