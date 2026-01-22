use std::ops::Range;
use log::error;
use wgpu::{Buffer, RenderPass};

pub struct Mesh {
    pub vertex_buf: Vec<Buffer>,
    pub index_buf: Vec<(Buffer, usize)>,
    pub layers_indices: Vec<Range<usize>>,

}

impl Mesh {
    pub fn new(v_buf: Vec<Buffer>,
               i_buf: Vec<(Buffer, usize)>,
               layers_indices: Vec<Range<usize>>) -> Self {
        Self {
            vertex_buf: v_buf,
            index_buf: i_buf,
            layers_indices
        }
    }

    pub fn render(&self, render_pass: &mut RenderPass, instances: &Range<u32>) {
        self.vertex_buf.iter().enumerate().for_each(|(i, v_buf)| {
            let (i_buf, _) = self.index_buf.get(i).unwrap();
            if v_buf.size() > 0 && i_buf.size() > 0 {
                render_pass.set_vertex_buffer(0, v_buf.slice(..));
                render_pass.set_index_buffer(i_buf.slice(..), wgpu::IndexFormat::Uint32);
                for range in &self.layers_indices {
                    let start = range.start;
                    let end = range.end;
                    // draw two instances, outlined and normal
                    render_pass.draw_indexed(start as u32..end as u32, 0, instances.clone());
                }
            } else {
                error!("Vertex/Index buffer are empty");
            }
        });
    }
}
