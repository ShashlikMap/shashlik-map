use std::sync::mpsc::channel;
use image::{ImageBuffer, Rgba};
use crate::RendererUpdateData;
use crate::collider::Collider;
use crate::render_config::RenderConfig;
use crate::styles::style_store::StyleStore;
use crate::texture_view_resources::TextureViewResources;
use crate::utils::{ReceiverExt, TextureExt};
use crate::view_projection::ViewProjection;
use crate::wgpu_canvas::WgpuCanvas;
use renderer_common::PreviewType;
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Buffer, Device};

pub(crate) struct GlobalContext {
    pub canvas: Box<dyn WgpuCanvas>,
    pub view_projection: ViewProjection,
    pub collider: Collider,
    pub styles_bind_group_layout: BindGroupLayout,
    pub style_bind_group: Option<BindGroup>,
    ssao_enabled: bool,
    pub x_real_mesh_shader_enabled: bool,
    pub(crate) texture_view_resources: TextureViewResources,
    preview_type: PreviewType,
    style_uniform_rx: tokio::sync::broadcast::Receiver<Vec<[[f32; 4]; 4]>>,
    png_buffer: Option<Buffer>
}

impl GlobalContext {
    pub fn new(canvas: Box<dyn WgpuCanvas>, render_config: &RenderConfig, style_store: &StyleStore) -> Self {
        let device = canvas.device();
        let view_projection = ViewProjection::new(device, render_config);
        let collider = Collider::new();
        let styles_bind_group_layout = Self::create_style_bind_group_layout(device);

        let texture_view_resources = TextureViewResources::new(render_config, device);
        GlobalContext {
            canvas,
            view_projection,
            collider,
            styles_bind_group_layout,
            style_bind_group: None,
            ssao_enabled: render_config.ssao_enabled,
            x_real_mesh_shader_enabled: render_config.x_real_mesh_shader_enabled,
            texture_view_resources,
            preview_type: render_config.preview_type,
            style_uniform_rx: style_store.subscribe(),
            png_buffer: None
        }
    }

    fn create_style_bind_group_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("styles_bind_group_layout"),
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.canvas.on_resize();
            let config = self.canvas.config();

            self.view_projection.resize(config.width, config.height);
        }
    }

    pub fn update(&mut self, render_config: &RenderConfig, data: RendererUpdateData) {
        self.view_projection.update(
            self.canvas.queue(),
            render_config,
            self.canvas.config(),
            data,
        );
        self.collider.update_view_proj(&self.view_projection);

        self.update_style_bind_group();

        self.preview_type = render_config.preview_type;
        self.ssao_enabled = render_config.ssao_enabled;
        self.x_real_mesh_shader_enabled = render_config.x_real_mesh_shader_enabled;
    }

    fn update_style_bind_group(&mut self) {
        let device = self.canvas.device();
        if let Ok(uniforms) = self.style_uniform_rx.no_lagged() {
            // TODO We could reuse the buffer if styles count has not changed
            let styles_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Style Buffer"),
                contents: bytemuck::cast_slice(&uniforms),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

            let styles_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.styles_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: styles_buffer.as_entire_binding(),
                }],
                label: Some("styles_bind_group"),
            });

            self.style_bind_group = Some(styles_bind_group);
        }
    }

    pub fn queue(&self) -> &wgpu::Queue {
        self.canvas.queue()
    }
    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        self.canvas.config()
    }
    pub fn device(&self) -> &wgpu::Device {
        self.canvas.device()
    }

    pub fn is_shadow_mapping_enabled(&self) -> bool {
        self.view_projection.is_shadow_mapping_enabled()
    }

    pub fn is_ssao_enabled(&self) -> bool {
        self.ssao_enabled
    }

    pub fn preview_type(&self) -> PreviewType {
        self.preview_type
    }

    pub fn set_png_buffer(&mut self, buffer: Buffer) {
        self.png_buffer = Some(buffer);
    }

    pub fn create_screenshot_if_available(&mut self) {
        if let Some(png_buffer) = self.png_buffer.take() {
            let buffer_slice = png_buffer.slice(..);
            let (tx, rx) = channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });

            self.device().poll(wgpu::PollType::wait_indefinitely()).unwrap();

            let texture = self.canvas.texture().unwrap();
            let texture_width = texture.width() as usize;
            let texture_height = texture.height() as usize;

            if let Ok(_) = rx.recv() {
                let data = buffer_slice.get_mapped_range().expect("Mapped range error");

                let mut packed_data = Vec::with_capacity(texture_width * texture_height * 4);

                for chunk in data.chunks_exact(texture.padded_bytes_per_row() as usize) {
                    packed_data.extend_from_slice(&chunk[..texture.unpadded_bytes_per_row() as usize]);
                }

                drop(data);
                png_buffer.unmap();

                if let Some(img_buf) = ImageBuffer::<Rgba<u8>, _>::from_raw(texture_width as u32, texture_height as u32, packed_data) {
                    img_buf.save("output.png").expect("PNG failed");
                    println!("PNG created");
                }
            }
        }
    }
}