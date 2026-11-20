use crate::global_context::GlobalContext;
use crate::mesh_layers::BaseMeshLayer;
use crate::pipelines::raster_pipeline::RasterPipeline;
use crate::textures::{create_simple_texture_with_data, TextureData};
use crate::vertex_attrs::{RasterInstance, RasterVertex};
use glam::DVec3;
use std::collections::HashMap;
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Buffer, RenderPass, Sampler, TextureFormat, TextureUsages};

/// Tile quad is subdivided, so it can follow the sphere curvature in Globe view
const GRID_SIZE: u32 = 16;

struct RasterTile {
    bind_group: BindGroup,
    origin: DVec3,
    world_size: f32,
}

/// World space raster tiles (e.g. rain radar), each tile has its own texture.
pub(crate) struct RasterLayer {
    pub texture_bind_group_layout: BindGroupLayout,
    sampler: Sampler,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
    instance_buffer: Option<Buffer>,
    tiles: HashMap<String, RasterTile>,
    // draw order: lower zoom (bigger) tiles first, so more detailed tiles are on top during transitions
    order: Vec<String>,
}

impl RasterLayer {
    pub fn new(global_context: &GlobalContext) -> Self {
        let device = global_context.device();
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("raster_texture_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("raster_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let (vertices, indices) = Self::grid();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("raster_grid_vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("raster_grid_indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            texture_bind_group_layout,
            sampler,
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            instance_buffer: None,
            tiles: HashMap::new(),
            order: Vec::new(),
        }
    }

    fn grid() -> (Vec<RasterVertex>, Vec<u32>) {
        let n = GRID_SIZE;
        let mut vertices = Vec::with_capacity(((n + 1) * (n + 1)) as usize);
        for y in 0..=n {
            for x in 0..=n {
                vertices.push(RasterVertex { uv: [x as f32 / n as f32, y as f32 / n as f32] });
            }
        }
        let mut indices = Vec::with_capacity((n * n * 6) as usize);
        for y in 0..n {
            for x in 0..n {
                let i = y * (n + 1) + x;
                let below = i + n + 1;
                indices.extend_from_slice(&[i, below, i + 1, i + 1, below, below + 1]);
            }
        }
        (vertices, indices)
    }

    /// `rgba` must be premultiplied alpha
    pub fn add(
        &mut self,
        key: String,
        origin: DVec3,
        world_size: f32,
        size: (u32, u32),
        rgba: &[u8],
        global_context: &GlobalContext,
    ) {
        let device = global_context.device();
        let texture_view = create_simple_texture_with_data(
            TextureData {
                sample_count: 1,
                size,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                // surface is non-sRGB, keep colors as they are
                format: TextureFormat::Rgba8Unorm,
            },
            global_context.queue(),
            device,
            rgba,
        );
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("raster_tile_bind_group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        self.tiles.insert(key, RasterTile { bind_group, origin, world_size });
        self.update_order();
    }

    fn update_order(&mut self) {
        let mut order: Vec<_> = self.tiles.keys().cloned().collect();
        order.sort_by(|a, b| self.tiles[b].world_size.total_cmp(&self.tiles[a].world_size));
        self.order = order;
    }

    pub fn render(
        &mut self,
        render_pass: &mut RenderPass,
        pipeline: &RasterPipeline,
        global_context: &GlobalContext,
    ) {
        let Some(instance_buffer) = self.instance_buffer.as_ref() else { return };
        if self.order.is_empty() {
            return;
        }
        pipeline.setup_render(render_pass, global_context);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        for (index, key) in self.order.iter().enumerate() {
            let index = index as u32;
            render_pass.set_bind_group(1, &self.tiles[key].bind_group, &[]);
            render_pass.draw_indexed(0..self.index_count, 0, index..index + 1);
        }
    }
}

impl BaseMeshLayer for RasterLayer {
    fn update(&mut self, global_context: &mut GlobalContext) {
        if self.order.is_empty() {
            return;
        }
        // positions are relative to the camera offset to keep f32 precision
        let cs_offset = global_context.view_projection.get_cs_offset();
        let instances: Vec<RasterInstance> = self
            .order
            .iter()
            .map(|key| {
                let tile = &self.tiles[key];
                let origin = tile.origin - cs_offset;
                RasterInstance { origin: [origin.x as f32, origin.y as f32], size: tile.world_size }
            })
            .collect();
        let data: &[u8] = bytemuck::cast_slice(&instances);

        let fits = self
            .instance_buffer
            .as_ref()
            .is_some_and(|buffer| buffer.size() >= data.len() as u64);
        if fits {
            global_context.queue().write_buffer(self.instance_buffer.as_ref().unwrap(), 0, data);
        } else {
            self.instance_buffer = Some(global_context.device().create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("raster_instances"),
                    contents: data,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                },
            ));
        }
    }

    fn clear_by_key(&mut self, key: &str) {
        if self.tiles.remove(key).is_some() {
            self.update_order();
        }
    }
}
