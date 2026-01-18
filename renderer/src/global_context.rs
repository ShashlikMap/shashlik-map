use wgpu::{BindGroup, BindGroupLayout, Device};
use cgmath::{Matrix4, Vector3};
use wgpu::util::DeviceExt;
use wgpu_canvas::wgpu_canvas::WgpuCanvas;
use crate::collision_handler::CollisionHandler;
use crate::consts::STYLE_SHADER_PARAMS_COUNT;
use crate::styles::style_store::StyleStore;
use crate::utils::ReceiverExt;
use crate::view_projection::ViewProjection;

pub struct GlobalContext {
    pub canvas: Box<dyn WgpuCanvas>,
    pub view_projection: ViewProjection,
    pub collision_handler: CollisionHandler,
    pub styles_bind_group_layout: BindGroupLayout,
    pub style_bind_group: Option<BindGroup>,
    style_uniform_rx: tokio::sync::broadcast::Receiver<Vec<[f32; STYLE_SHADER_PARAMS_COUNT]>>,
}

impl GlobalContext {
    pub fn new(canvas: Box<dyn WgpuCanvas>, style_store: &StyleStore) -> Self {
        let device = canvas.device();
        let config = canvas.config();
        let view_projection = ViewProjection::new(device);
        let collision_handler = CollisionHandler::new(config.width as f32, config.height as f32);
        let styles_bind_group_layout = Self::create_style_bind_group_layout(device);
        GlobalContext {
            canvas,
            view_projection,
            collision_handler,
            styles_bind_group_layout,
            style_bind_group: None,
            style_uniform_rx: style_store.subscribe(),
        }
    }

    fn create_style_bind_group_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
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
            self.collision_handler
                .resize(config.width as f32, config.height as f32);
        }
    }

    pub fn update(&mut self, view_proj_matrix: Matrix4<f64>, cs_offset: Vector3<f64>) {
        self.view_projection.update(
            self.canvas.queue(),
            self.canvas.config(),
            view_proj_matrix,
            cs_offset,
        );

        self.update_style_bind_group();
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
}