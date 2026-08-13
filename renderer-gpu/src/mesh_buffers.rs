use std::sync::atomic::{AtomicUsize, Ordering};
use wgpu::Buffer;

#[derive(Clone, Default)]
pub struct MeshBuffers {
    instance_buffer: Option<BufferWithId>,
    culled_buffer: Option<BufferWithId>,
    instance_args_buffer: Option<BufferWithId>,
}

impl MeshBuffers {
    pub fn builder() -> MeshBuffers {
        Self::default()
    }

    pub fn with_instance_buffer(mut self, buffer: Option<Buffer>) -> Self {
        self.instance_buffer = buffer.map(Into::into);
        self
    }

    pub fn with_culled_and_args_buffer(mut self, culled_buffer: Option<Buffer>, args_buffer: Option<Buffer>) -> Self {
        self.culled_buffer = culled_buffer.map(Into::into);
        self.instance_args_buffer = args_buffer.map(Into::into);
        self
    }

    pub fn instance_buffer(&self) -> Option<&Buffer> {
        self.instance_buffer.as_ref().map(|id| &id.buffer)
    }

    pub fn culled_buffer(&self) -> Option<&Buffer> {
        self.culled_buffer.as_ref().map(|id| &id.buffer)
    }

    pub fn args_buffer(&self) -> Option<&Buffer> {
        self.instance_args_buffer.as_ref().map(|id| &id.buffer)
    }

    pub fn instance_buffer_with_id(&self) -> Option<&BufferWithId> {
        self.instance_buffer.as_ref()
    }

    pub fn culled_buffer_with_id(&self) -> Option<&BufferWithId> {
        self.culled_buffer.as_ref()
    }

    pub fn args_buffer_with_id(&self) -> Option<&BufferWithId> {
        self.instance_args_buffer.as_ref()
    }
}

static BUFFER_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
pub struct BufferWithId {
    id: usize,
    buffer: Buffer,
}

impl BufferWithId {
    pub fn new(buffer: Buffer) -> Self {
        let id = BUFFER_ID.fetch_add(1, Ordering::Relaxed);
        // println!("buf id = {id}");
        Self {
            id,
            buffer,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
}

impl From<Buffer> for BufferWithId {
    fn from(value: Buffer) -> Self {
        BufferWithId::new(value)
    }
}

