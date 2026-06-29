use crate::collider::Collider;
use crate::styles::style_store::StyleStore;
use crate::utils::ReceiverExt;
use crate::view_projection::ViewProjection;
use crate::RendererUpdateData;
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Device};
use wgpu_canvas::wgpu_canvas::WgpuCanvas;

pub struct GlobalContext {
    pub view_projection: ViewProjection,
    pub collider: Collider,
    // pub styles_bind_group_layout: BindGroupLayout,
    pub style_bind_group: Option<BindGroup>,
    pub is_preview_render: bool,
    pub is_g_buffer_render: bool,
    pub is_shadow_render: bool,
    style_uniform_rx: tokio::sync::broadcast::Receiver<Vec<[[f32; 4]; 4]>>,
}

impl GlobalContext {
    pub fn new(style_store: &StyleStore) -> Self {
        let view_projection = ViewProjection::new();
        let collider = Collider::new();
        // let styles_bind_group_layout = Self::create_style_bind_group_layout(device);

        #[cfg(target_os = "macos")]
        // let ssao_size = (canvas.config().width, canvas.config().height);
        #[cfg(not(target_os = "macos"))]
        let mut ssao_size = (canvas.config().width / 2, canvas.config().height / 2);

        // let ssao_texture = create_simple_texture(
        //     TextureData {
        //         sample_count: 1,
        //         size: ssao_size,
        //         usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
        //         format: TextureFormat::Rgba16Float,
        //     },
        //     device,
        // );
        // let shadow_map_depth_texture = create_depth_texture(unsafe { SHADOWS_TEX_SIZE },
        //                                                     1,
        //                                                     TextureFormat::Depth32Float,
        //                                                     device);
        GlobalContext {
            view_projection,
            collider,
            // styles_bind_group_layout,
            style_bind_group: None,
            is_preview_render: false,
            is_g_buffer_render: false,
            is_shadow_render: false,
            // ssao_texture,
            // shadow_map_depth_texture,
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
            // self.canvas.on_resize();
            // let config = self.canvas.config();

            self.view_projection.resize(500, 500);
        }
    }

    pub fn update(&mut self, data: RendererUpdateData) {
        self.view_projection.update(
            // self.canvas.queue(),
            // self.canvas.config(),
            data
        );
        // self.collider.update_view_proj(&self.view_projection);

        // self.update_style_bind_group();
    }

    fn update_style_bind_group(&mut self) {
        // let device = self.canvas.device();
        // if let Ok(uniforms) = self.style_uniform_rx.no_lagged() {
        //     // TODO We could reuse the buffer if styles count has not changed
        //     let styles_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //         label: Some("Style Buffer"),
        //         contents: bytemuck::cast_slice(&uniforms),
        //         usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        //     });
        //
        //     let styles_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //         layout: &self.styles_bind_group_layout,
        //         entries: &[wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: styles_buffer.as_entire_binding(),
        //         }],
        //         label: Some("styles_bind_group"),
        //     });
        //
        //     self.style_bind_group = Some(styles_bind_group);
        // }
    }

    pub fn queue(&self) -> &wgpu::Queue {
        panic!("QUEUE!");
        // self.canvas.queue()
    }
    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        panic!("QUEUE!");
        // self.canvas.config()
    }
    pub fn device(&self) -> &wgpu::Device {
        panic!("QUEUE!");
        // self.canvas.device()
    }
}