use wgpu::Buffer;

pub struct Mesh {
    pub vertex_buf: Vec<Buffer>,
    pub index_buf: Vec<(Buffer, usize)>,
}

impl Mesh {
    pub fn new(v_buf: Vec<Buffer>, i_buf: Vec<(Buffer, usize)>) -> Self {
        Self {
            vertex_buf: v_buf,
            index_buf: i_buf,
        }
    }
}
