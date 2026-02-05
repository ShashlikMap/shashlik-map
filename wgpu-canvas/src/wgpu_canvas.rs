use wgpu::{Texture, TextureView};

pub trait WgpuCanvas: Send + Sync {
    fn queue(&self) -> &wgpu::Queue;
    fn config(&self) -> &wgpu::SurfaceConfiguration;
    fn device(&self) -> &wgpu::Device;
    fn create_texture_view(&mut self) -> TextureView;
    fn present(&mut self) -> Option<Texture>;
    fn on_resize(&mut self);
}
