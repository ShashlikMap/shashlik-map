use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};
use wgpu::Buffer;
use crate::mesh::InstanceBuffer;
use crate::mesh::mesh_instance_input::MeshInstanceInput;

pub struct MeshBuffers<I: MeshInstanceInput> {
    instance_buffer: Option<BufferWithId>,
    culled_buffer: Option<BufferWithId>,
    instance_args_buffer: Option<BufferWithId>,
    _phantom_data: PhantomData<I>,
}

impl<I: MeshInstanceInput> Clone for MeshBuffers<I> {
    fn clone(&self) -> Self {
        MeshBuffers::<I> {
            instance_buffer: self.instance_buffer.clone(),
            culled_buffer: self.culled_buffer.clone(),
            instance_args_buffer: self.instance_args_buffer.clone(),
            _phantom_data: PhantomData,
        }
    }
}

impl<I: MeshInstanceInput> Default for MeshBuffers<I> {
    fn default() -> Self {
        MeshBuffers::<I> {
            instance_buffer: None,
            culled_buffer: None,
            instance_args_buffer: None,
            _phantom_data: PhantomData,
        }
    }
}

impl<I: MeshInstanceInput> MeshBuffers<I> {
    pub fn builder() -> MeshBuffers<I> {
        Self::default()
    }

    pub fn with_instance_buffer(mut self, instance_buffer: &InstanceBuffer<I>) -> Self {
        self.instance_buffer = instance_buffer.buffer.clone().map(Into::into);
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
pub(crate) type UniqueBufferId = usize;

#[derive(Clone)]
pub struct BufferWithId {
    id: UniqueBufferId,
    buffer: Buffer,
}

impl BufferWithId {
    pub fn new(buffer: Buffer) -> Self {
        let id = BUFFER_ID.fetch_add(1, Ordering::Relaxed);
        Self {
            id,
            buffer,
        }
    }

    pub fn id(&self) -> UniqueBufferId {
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

