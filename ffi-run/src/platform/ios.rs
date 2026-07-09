use app_surface::{AppSurface, IOSViewObj};
use wgpu::{CurrentSurfaceTexture, Device, Queue, SurfaceConfiguration, SurfaceTexture, Texture, TextureView};
use crate::ShashlikMapApi;
use map::tiles::default_tiles_provider::DefaultTilesProvider;
use osm::source::reqwest_source::ReqwestSource;
use map::{ShashlikMap, DEFAULT_FONT};
use std::sync::RwLock;
use std::ffi::c_void;
use std::mem;
use osm::tiles::TileStore;
use objc::runtime::Object;
use app_surface::SurfaceFrame;
use map::feature_processor::ShashlikFeatureProcessor;
use renderer_gpu::GpuRenderer;
use renderer_gpu::wgpu_canvas::WgpuCanvas;
use renderer_common::feature_layer_tags;

extern "C" fn ios_callback_stub(_arg: i32) {}

#[uniffi::export]
pub fn create_shashlik_map_api_for_ios(view: u64, metal_layer: u64, maximum_frames: i32, _tiles_db: String) -> ShashlikMapApi {
	let ios_view_obj = IOSViewObj {
		view: view as *mut Object,
		metal_layer: metal_layer as *mut c_void,
		maximum_frames,
		callback_to_swift: ios_callback_stub,
	};
	let app_surface = AppSurface::new(ios_view_obj);
	let wrapper = IOSPlatformAppSurface { app_surface, surface_texture: None };
	let reqwest_source = ReqwestSource::new();
	let tile_store = Box::new(TileStore::new(reqwest_source));
	let feature_processor = ShashlikFeatureProcessor::new();
	// TODO DPI from iOS
	let shashlik_map = pollster::block_on(async {
		let renderer = GpuRenderer::new(feature_layer_tags(),
		                                Box::new(wrapper), &DEFAULT_FONT).await?;
		ShashlikMap::new(renderer,
		                 DefaultTilesProvider::new(tile_store, feature_processor, 1.35),
		).await
	}).unwrap();
	ShashlikMapApi { shashlik_map: RwLock::new(shashlik_map) }
}

pub struct IOSPlatformAppSurface {
	pub app_surface: AppSurface,
	surface_texture: Option<SurfaceTexture>,
}

// SAFETY: Under iOS we ensure AppSurface only used on main thread for rendering operations.
unsafe impl Send for IOSPlatformAppSurface {}
unsafe impl Sync for IOSPlatformAppSurface {}

impl WgpuCanvas for IOSPlatformAppSurface {
	fn queue(&self) -> &Queue { &self.app_surface.queue }
	fn config(&self) -> &SurfaceConfiguration { &self.app_surface.config }
	fn device(&self) -> &Device { &self.app_surface.device }
	fn create_texture_view(&mut self) -> TextureView {
		let surface_texture = match self.app_surface.surface.get_current_texture() {
			CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
			_ => panic!("Failed to acquire next swap chain texture!"),
		};

		let texture_view = surface_texture
			.texture
			.create_view(&wgpu::TextureViewDescriptor::default());
		self.surface_texture = Some(surface_texture);
		texture_view
	}

	fn present(&mut self) -> Option<Texture> {
		if let Some(surface_texture) = mem::take(&mut self.surface_texture) {
			surface_texture.present();
		}
		None
	}
	
	fn on_resize(&mut self) {
		self.app_surface.resize_surface();
	}
}