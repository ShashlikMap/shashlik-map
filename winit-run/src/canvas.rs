use wgpu::wgt::TextureViewDescriptor;
use wgpu::{Device, Queue, SurfaceConfiguration, Texture, TextureView};
use wgpu_canvas::wgpu_canvas::WgpuCanvas;

pub struct SlintWgpuCanvas(pub Queue, pub Device, pub SurfaceConfiguration, pub Texture);

impl WgpuCanvas for SlintWgpuCanvas {
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
}
