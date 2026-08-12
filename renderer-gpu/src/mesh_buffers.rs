use wgpu::Buffer;

#[derive(Clone, Default)]
pub struct MeshBuffers {
    pub instance_buffer: Option<Buffer>,
    pub culled_buffer: Option<Buffer>,
    pub instance_args_buffer: Option<Buffer>,
}

impl MeshBuffers {
    pub fn with_instance_buffer(buffer: Option<Buffer>) -> Self {
        Self {
            instance_buffer: buffer,
            ..Default::default()
        }
    }
}