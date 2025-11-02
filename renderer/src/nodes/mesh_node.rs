use crate::mesh::mesh::Mesh;
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::SceneNode;
use crate::vertex_attrs::InstancePos;
use crate::{GlobalContext, ReceiverExt};
use cgmath::Vector3;
use log::error;
use std::ops::Range;
use tokio::sync::broadcast::Receiver;
use wgpu::util::DeviceExt;
use wgpu::{Buffer, Device, Queue, RenderPass};

pub struct PositionedMesh {
    mesh: Mesh,
    instance_buffer: Buffer,
    attrs: Vec<InstancePos>,
    original_positions: Vec<Vector3<f32>>,
    is_two_instances: bool,
    spatial_rx: Receiver<SpatialData>,
    original_spatial_data: SpatialData,
    color_alpha: f32,
}

impl Mesh {
    pub fn to_positioned(
        self,
        device: &Device,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
    ) -> PositionedMesh {
        PositionedMesh::new(
            device,
            self,
            vec![Vector3::new(0.0, 0.0, 0.0)],
            spatial_rx,
            false,
        )
    }
    pub fn to_positioned_with_instances(
        self,
        device: &Device,
        original_positions: Vec<Vector3<f32>>,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        is_two_instances: bool,
    ) -> PositionedMesh {
        PositionedMesh::new(
            device,
            self,
            original_positions,
            spatial_rx,
            is_two_instances,
        )
    }

    fn render(&self, render_pass: &mut RenderPass, instances: &Range<u32>) {
        self.vertex_buf.iter().enumerate().for_each(|(i, v_buf)| {
            let (i_buf, _) = self.index_buf.get(i).unwrap();
            if v_buf.size() > 0 && i_buf.size() > 0 {
                render_pass.set_vertex_buffer(0, v_buf.slice(..));
                render_pass.set_index_buffer(i_buf.slice(..), wgpu::IndexFormat::Uint32);
                let mut prev = 0u32;
                for to_index in &self.layers_indices {
                    // draw two instances, outlined and normal
                    render_pass.draw_indexed(prev..*to_index as u32, 0, instances.clone());
                    prev = *to_index as u32;
                }
            } else {
                error!("Vertex/Index buffer are empty");
            }
        });
    }
}

impl PositionedMesh {
    pub fn new(
        device: &Device,
        mesh: Mesh,
        original_positions: Vec<Vector3<f32>>,
        mut spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        is_two_instances: bool,
    ) -> Self {
        let spatial_data = spatial_rx.try_recv().unwrap_or(SpatialData::new());
        let mut attrs = Vec::new();

        Self::update_attrs(
            &mut attrs,
            &original_positions,
            &spatial_data,
            is_two_instances,
            1.0,
        );

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            // TODO It probably should be configurable, so it would be possible to draw two or more instances.
            contents: bytemuck::cast_slice(attrs.as_slice()),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            mesh,
            instance_buffer,
            attrs,
            original_positions,
            is_two_instances,
            spatial_rx,
            color_alpha: 1.0,
            original_spatial_data: spatial_data,
        }
    }
}

impl PositionedMesh {
    fn update_attrs(
        attrs: &mut Vec<InstancePos>,
        original_positions: &Vec<Vector3<f32>>,
        spatial_data: &SpatialData,
        is_two_instances: bool,
        color_alpha: f32,
    ) {
        attrs.clear();
        for i in 0..original_positions.len() {
            let instance_pos = InstancePos {
                position: (original_positions[i] + spatial_data.transform).into(),
                color_alpha,
            };
            attrs.push(instance_pos);
            if is_two_instances {
                attrs.push(instance_pos);
            }
        }
    }
}

impl SceneNode for Mesh {
    fn render(&self, render_pass: &mut RenderPass, _global_context: &mut GlobalContext) {
        self.render(render_pass, &(0..1));
    }
}

impl SceneNode for PositionedMesh {
    fn update(
        &mut self,
        _device: &Device,
        queue: &Queue,
        _config: &wgpu::SurfaceConfiguration,
        _global_context: &mut GlobalContext,
    ) {
        self.color_alpha -= 0.01;
        if self.color_alpha < 0.0 {
            self.color_alpha = 1.0;
        }

        if let Ok(spatial_data) = self.spatial_rx.no_lagged() {
            self.original_spatial_data = spatial_data;
            // Self::update_attrs(
            //     &mut self.attrs,
            //     &self.original_positions,
            //     &spatial_data,
            //     self.is_two_instances,
            //     self.color_alpha
            // );
            //
            // queue.write_buffer(
            //     &self.instance_buffer,
            //     0,
            //     bytemuck::cast_slice(self.attrs.as_slice()),
            // );
        }

        Self::update_attrs(
            &mut self.attrs,
            &self.original_positions,
            &self.original_spatial_data,
            self.is_two_instances,
            self.color_alpha,
        );

        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(self.attrs.as_slice()),
        );
    }

    fn render(&self, render_pass: &mut RenderPass, _global_context: &mut GlobalContext) {
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        let range = 0u32..self.attrs.len() as u32;
        self.mesh.render(render_pass, &range);
    }
}
