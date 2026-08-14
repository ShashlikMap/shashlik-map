use crate::RendererUpdateData;
use crate::collider::Collider;
use crate::render_config::RenderConfig;
use crate::styles::style_store::StyleStore;
use crate::texture_view_resources::TextureViewResources;
use crate::utils::ReceiverExt;
use crate::view_projection::ViewProjection;
use crate::wgpu_canvas::WgpuCanvas;
use renderer_common::PreviewType;
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Device};

#[derive(Eq, PartialEq)]
pub(crate) enum GlobalRenderStep {
    MainStep,
    PreviewStep,
}

pub struct GlobalContext {
    pub canvas: Box<dyn WgpuCanvas>,
    pub view_projection: ViewProjection,
    pub collider: Collider,
    pub styles_bind_group_layout: BindGroupLayout,
    pub style_bind_group: Option<BindGroup>,
    pub(crate) render_step: GlobalRenderStep,
    ssao_enabled: bool,
    pub(crate) texture_view_resources: TextureViewResources,
    preview_type: PreviewType,
    style_uniform_rx: tokio::sync::broadcast::Receiver<Vec<[[f32; 4]; 4]>>,
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
            render_step: GlobalRenderStep::MainStep,
            ssao_enabled: render_config.ssao_enabled,
            texture_view_resources,
            preview_type: render_config.preview_type,
            style_uniform_rx: style_store.subscribe(),
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

    pub(crate) fn check_render_step(&self, step: GlobalRenderStep) -> bool {
        self.render_step == step
    }
}