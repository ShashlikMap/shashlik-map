use bytemuck::NoUninit;
use rustc_hash::FxHashMap;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{Buffer, BufferAddress, BufferUsages, Device, Queue, COPY_BUFFER_ALIGNMENT};

pub struct BufferPool {
    recycled: Vec<Buffer>,
    used: FxHashMap<String, Vec<Buffer>>,
}

impl BufferPool {
    pub fn new() -> Self {
        Self {
            recycled: Vec::new(),
            used: FxHashMap::default(),
        }
    }
    pub fn create<'a, T: NoUninit>(
        &mut self,
        device: &Device,
        queue: &Queue,
        key: Option<&str>,
        label: &'static str,
        usage: BufferUsages,
        data: &'a [T],
    ) -> Buffer {
        let data = bytemuck::cast_slice(data);

        // special case, early exit
        if key.is_none() {
            return Self::create_new(device, label, usage, data);
        }

        let actual_usage = usage | BufferUsages::COPY_DST;
        let padded_size = self.padded_size(data);
        let new_buffer_index = self
            .recycled
            .iter()
            .enumerate()
            .filter(|(_, item)| item.size() >= padded_size && item.usage() == actual_usage)
            .min_by_key(|(_, item)| item.size())
            .map(|(index, _)| index);
        let new_buffer = new_buffer_index.map(|index| self.recycled.swap_remove(index));

        let buffer = if let Some(value) = new_buffer {
            queue.write_buffer(&value, 0, data);
            value
        } else {
            Self::create_new(device, label, actual_usage, data)
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

    fn create_new(
        device: &Device,
        label: &str,
        usage: BufferUsages,
        data: &[u8],
    ) -> Buffer {
        device.create_buffer_init(&BufferInitDescriptor {
            label: Some(label),
            contents: data,
            usage,
        })
    }

    /// @see device.create_buffer_init. It uses the same approach to calculate padded size
    fn padded_size(&self, data: &[u8]) -> BufferAddress {
        let unpadded_size = data.len() as BufferAddress;
        let align_mask = COPY_BUFFER_ALIGNMENT - 1;
        let padded_size = ((unpadded_size + align_mask) & !align_mask).max(COPY_BUFFER_ALIGNMENT);
        padded_size
    }
}
