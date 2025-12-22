use app_surface::{AppSurface, IOSViewObj};
use wgpu::{Device, Queue, SurfaceConfiguration, SurfaceError, SurfaceTexture};
use wgpu_canvas::wgpu_canvas::WgpuCanvas;
use crate::ShashlikMapApi;
use map::tiles::shashlik_tiles_provider_v0::ShashlikTilesProviderV0;
use osm::source::reqwest_source::ReqwestSource;
use map::ShashlikMap;
use std::sync::RwLock;
use std::ffi::c_void;
use objc::runtime::Object;
use app_surface::SurfaceFrame;
use map::feature_processor::ShashlikFeatureProcessor;

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
	let wrapper = IOSPlatformAppSurface { app_surface };
	let reqwest_source = ReqwestSource::new();
	let feature_processor = ShashlikFeatureProcessor::new();
	// TODO DPI from iOS
	let shashlik_map = pollster::block_on(ShashlikMap::new(Box::new(wrapper), ShashlikTilesProviderV0::new(reqwest_source, feature_processor, 1.35))).unwrap();
	ShashlikMapApi { shashlik_map: RwLock::new(shashlik_map) }
}

pub struct IOSPlatformAppSurface {
	pub app_surface: AppSurface,
}

// SAFETY: Under iOS we ensure AppSurface only used on main thread for rendering operations.
unsafe impl Send for IOSPlatformAppSurface {}
unsafe impl Sync for IOSPlatformAppSurface {}

impl WgpuCanvas for IOSPlatformAppSurface {
	fn queue(&self) -> &Queue { &self.app_surface.queue }
	fn config(&self) -> &SurfaceConfiguration { &self.app_surface.config }
	fn device(&self) -> &Device { &self.app_surface.device }
	fn get_current_texture(&self) -> Result<SurfaceTexture, SurfaceError> {
		self.app_surface.surface.get_current_texture()
	}
	fn on_resize(&mut self) {
		self.app_surface.resize_surface();
	}
	fn on_pre_render(&self) {}
	fn on_post_render(&self) {}
}