use crate::mesh::mesh::Mesh;
use crate::modifier::render_modifier::SpatialData;
use crate::nodes::SceneNode;
use crate::vertex_attrs::InstancePos;
use crate::{GlobalContext, ReceiverExt};
use cgmath::Vector3;
use cgmath::num_traits::clamp;
use geo_types::point;
use log::error;
use rstar::primitives::Rectangle;
use std::ops::Range;
use tokio::sync::broadcast::Receiver;
use wgpu::util::DeviceExt;
use wgpu::{Buffer, Device, Queue, RenderPass};

pub struct PositionedMesh {
    mesh: Mesh,
    instance_buffer: Buffer,
    attrs: Vec<InstancePos>,
    original_positions_alpha: Vec<(Vector3<f32>, f32)>, // TODO Proper structure with bound
    is_two_instances: bool,
    spatial_rx: Receiver<SpatialData>,
    original_spatial_data: SpatialData,
    with_collisions: bool,
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
            false,
        )
    }
    pub fn to_positioned_with_instances(
        self,
        device: &Device,
        original_positions: Vec<Vector3<f32>>,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        is_two_instances: bool,
        with_collisions: bool,
    ) -> PositionedMesh {
        PositionedMesh::new(
            device,
            self,
            original_positions,
            spatial_rx,
            is_two_instances,
            with_collisions,
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
        with_collisions: bool,
    ) -> Self {
        let original_positions_alpha = original_positions.iter().map(|v| (*v, 1.0)).collect();
        let spatial_data = spatial_rx.try_recv().unwrap_or(SpatialData::new());
        let mut attrs = Vec::new();

        Self::update_attrs(
            &mut attrs,
            &original_positions_alpha,
            &spatial_data,
            is_two_instances,
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
            original_positions_alpha,
            is_two_instances,
            spatial_rx,
            original_spatial_data: spatial_data,
            with_collisions,
        }
    }
}

impl PositionedMesh {
    fn update_attrs(
        attrs: &mut Vec<InstancePos>,
        original_positions_alpha: &Vec<(Vector3<f32>, f32)>,
        spatial_data: &SpatialData,
        is_two_instances: bool,
    ) {
        attrs.clear();
        for i in 0..original_positions_alpha.len() {
            let item = original_positions_alpha[i];
            let instance_pos = InstancePos {
                position: (item.0 + spatial_data.transform).into(),
                color_alpha: item.1,
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
        config: &wgpu::SurfaceConfiguration,
        global_context: &mut GlobalContext,
    ) {
        if self.with_collisions {
            let screen_position_calculator = global_context
                .camera_controller
                .borrow()
                .screen_position_calculator(config);

            for item in &mut self.original_positions_alpha {
                let screen_pos = screen_position_calculator.screen_position(Vector3::new(
                    item.0.x + self.original_spatial_data.transform.x,
                    item.0.y + self.original_spatial_data.transform.y,
                    0.0,
                ));
                let bounds = Rectangle::from_corners(
                    point! { x: screen_pos.x - 20.0, y: screen_pos.y - 20.0},
                    point! { x: screen_pos.x + 20.0, y: screen_pos.y + 20.0},
                );

                if let Some(added) = global_context.collision_handler.insert(config, bounds) {
                    if added {
                        item.1 = clamp(item.1 + 0.05, 0.0, 1.0);
                    } else {
                        item.1 = clamp(item.1 - 0.05, 0.0, 1.0);
                    };
                }
            }
        }

        let mut update_attrs = self.with_collisions;

        if let Ok(spatial_data) = self.spatial_rx.no_lagged() {
            self.original_spatial_data = spatial_data;
            update_attrs = true;
        }

        if update_attrs {
            Self::update_attrs(
                &mut self.attrs,
                &self.original_positions_alpha,
                &self.original_spatial_data,
                self.is_two_instances,
            );

            queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(self.attrs.as_slice()),
            );
        }
    }

    fn render(&self, render_pass: &mut RenderPass, _global_context: &mut GlobalContext) {
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        let range = 0u32..self.attrs.len() as u32;
        self.mesh.render(render_pass, &range);
    }
}
