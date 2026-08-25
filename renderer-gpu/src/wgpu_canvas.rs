use wgpu::{Device, Queue, SurfaceColorSpace, SurfaceConfiguration, Texture, TextureUsages, TextureView};
use wgpu::wgt::TextureViewDescriptor;

pub trait WgpuCanvas: Send + Sync {
    fn queue(&self) -> &wgpu::Queue;
    fn config(&self) -> &wgpu::SurfaceConfiguration;
    fn device(&self) -> &wgpu::Device;
    fn create_texture_view(&mut self) -> TextureView;
    fn present(&mut self) -> Option<Texture>;
    fn on_resize(&mut self);

    fn texture(&self) -> Option<&Texture> { None } 
}


pub struct DefaultWgpuCanvas {
    queue: Queue,
    device: Device,
    texture: Texture,
    config: wgpu::SurfaceConfiguration,
}

impl DefaultWgpuCanvas {
    pub fn new(queue: Queue, device: Device, texture: Texture) -> Self {
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: texture.format(),
            color_space: SurfaceColorSpace::Auto,
            width: texture.width(),
            height: texture.height(),
            present_mode: Default::default(),
            desired_maximum_frame_latency: 2,
            alpha_mode: Default::default(),
            view_formats: vec![],
        };
        Self {
            queue,
            device,
            texture,
            config,
        }
    }
}

impl WgpuCanvas for DefaultWgpuCanvas {
    fn queue(&self) -> &Queue {
        &self.queue
    }

    fn config(&self) -> &SurfaceConfiguration {
        &self.config
    }

    fn device(&self) -> &Device {
        &self.device
    }

    fn create_texture_view(&mut self) -> TextureView {
        self.texture.create_view(&TextureViewDescriptor::default())
    }

    fn present(&mut self) -> Option<Texture> {
        Some(self.texture.clone())
    }

    fn on_resize(&mut self) {}

    fn texture(&self) -> Option<&Texture> {
        Some(&self.texture)
    }
}