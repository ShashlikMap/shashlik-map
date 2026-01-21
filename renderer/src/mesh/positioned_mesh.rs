use crate::global_context::GlobalContext;
use crate::mesh::mesh::Mesh;
use crate::mesh::mesh_instance_input::MeshInstanceInput;
use crate::modifier::render_modifier::SpatialData;
use crate::utils::ReceiverExt;
use cgmath::Vector3;
use cgmath::num_traits::clamp;
use geo_types::point;
use log::error;
use rstar::primitives::Rectangle;
use std::ops::Range;
use tokio::sync::broadcast::Receiver;
use wgpu::util::DeviceExt;
use wgpu::{Buffer, RenderPass};

pub struct PositionedMesh<T: MeshInstanceInput> {
    mesh: Mesh,
    instance_buffer: Option<Buffer>,
    attrs: Vec<T>,
    instance_positions_and_alpha: Vec<(Vector3<f64>, f32)>, // TODO Proper structure with bound
    cs_offset: Vector3<f64>,
    is_two_instances: bool,
    spatial_rx: Receiver<SpatialData>,
    original_spatial_data: SpatialData,
    with_collisions: bool,
    first_render: bool,
}

impl Mesh {
    pub fn to_positioned<T: MeshInstanceInput>(
        self,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
    ) -> PositionedMesh<T> {
        PositionedMesh::new(self, None, spatial_rx, false, false)
    }
    pub fn to_positioned_with_instances<T: MeshInstanceInput>(
        self,
        instance_positions: Option<Vec<Vector3<f64>>>,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        is_two_instances: bool,
        with_collisions: bool,
    ) -> PositionedMesh<T> {
        PositionedMesh::new(
            self,
            instance_positions,
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

impl<T: MeshInstanceInput> PositionedMesh<T> {
    pub fn new(
        mesh: Mesh,
        instance_positions: Option<Vec<Vector3<f64>>>,
        spatial_rx: tokio::sync::broadcast::Receiver<SpatialData>,
        is_two_instances: bool,
        with_collisions: bool,
    ) -> Self {
        let instance_positions_and_alpha = instance_positions
            .unwrap_or(vec![Vector3::new(0.0, 0.0, 0.0)])
            .iter()
            .map(|v| (*v, 1.0))
            .collect();
        Self {
            mesh,
            instance_buffer: None,
            attrs: vec![],
            instance_positions_and_alpha,
            cs_offset: Vector3::new(0.0, 0.0, 0.0),
            is_two_instances,
            spatial_rx,
            original_spatial_data: SpatialData::new(),
            with_collisions,
            first_render: true,
        }
    }

    pub fn update(&mut self, global_context: &mut GlobalContext) {
        if self.with_collisions {
            for item in &mut self.instance_positions_and_alpha {
                let screen_pos = global_context.view_projection.screen_position(Vector3::new(
                    item.0.x + self.original_spatial_data.transform.x,
                    item.0.y + self.original_spatial_data.transform.y,
                    0.0,
                ));
                // TODO Bounds for svg?
                // no need to use f64 for collision detection
                let bounds = Rectangle::from_corners(
                    point! { x: screen_pos.x as f32 - 20.0, y: screen_pos.y as f32 - 20.0},
                    point! { x: screen_pos.x as f32+ 20.0, y: screen_pos.y as f32 + 20.0},
                );

                let within_screen = global_context.collision_handler.within_screen(bounds);
                if within_screen {
                    if global_context.collision_handler.insert(bounds) {
                        item.1 = clamp(item.1 + 0.05, 0.0, 1.0);
                    } else {
                        if self.first_render {
                            item.1 = 0.0;
                        } else {
                            item.1 = clamp(item.1 - 0.05, 0.0, 1.0);
                        }
                    }
                }
            }
        }

        self.first_render = false;

        let cs_offset_updated = global_context.view_projection.cs_offset != self.cs_offset;
        self.cs_offset = global_context.view_projection.cs_offset;
        let mut update_attrs = self.with_collisions || cs_offset_updated;

        if let Ok(spatial_data) = self.spatial_rx.no_lagged() {
            self.original_spatial_data = spatial_data;
            update_attrs = true;
        }

        if update_attrs {
            let prev_attr_len = self.attrs.len();
            T::fill_attrs(
                &mut self.attrs,
                &self.cs_offset,
                &self.instance_positions_and_alpha,
                &self.original_spatial_data,
                self.is_two_instances,
            );
            if self.attrs.len() != prev_attr_len {
                self.instance_buffer = Some(global_context.device().create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label: Some("Instance Buffer"),
                        // TODO It probably should be configurable, so it would be possible to draw two or more instances.
                        contents: bytemuck::cast_slice(self.attrs.as_slice()),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    },
                ));
            } else {
                let queue = global_context.queue();
                queue.write_buffer(
                    self.instance_buffer
                        .as_ref()
                        .expect("Buffer should be created"),
                    0,
                    bytemuck::cast_slice(self.attrs.as_slice()),
                );
            }
        }
    }

    pub fn render(&mut self, render_pass: &mut RenderPass) {
        render_pass.set_vertex_buffer(
            1,
            self.instance_buffer
                .as_ref()
                .expect("Buffer should be created")
                .slice(..),
        );
        let range = 0u32..self.attrs.len() as u32;
        self.mesh.render(render_pass, &range);
    }
}
