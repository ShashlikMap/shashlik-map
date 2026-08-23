use wgpu::{Device, Queue, SurfaceConfiguration, Texture, TextureView};
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


pub struct DefaultWgpuCanvas(pub Queue, pub Device, pub SurfaceConfiguration, pub Texture);

impl WgpuCanvas for DefaultWgpuCanvas {
    fn queue(&self) -> &Queue {
        &self.0
    }

    fn config(&self) -> &SurfaceConfiguration {
        &self.2
    }

    fn device(&self) -> &Device {
        &self.1
    }

    fn create_texture_view(&mut self) -> TextureView {
        self.3.create_view(&TextureViewDescriptor::default())
    }

    fn present(&mut self) -> Option<Texture> {
        Some(self.3.clone())
    }

    fn on_resize(&mut self) {}

    fn texture(&self) -> Option<&Texture> {
        Some(&self.3)
    }
}