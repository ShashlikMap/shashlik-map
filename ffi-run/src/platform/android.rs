use crate::ShashlikMapApi;
use app_surface::AppSurface;
use app_surface::SurfaceFrame;
use jni::JNIEnv;
use jni::objects::JClass;
use jni::objects::JString;
use jni::sys::jfloat;
use jni::sys::{jboolean, jlong, jobject};
use jni_fn::jni_fn;
use map::ShashlikMap;
use map::feature_processor::ShashlikFeatureProcessor;
use map::tiles::shashlik_tiles_provider_v0::ShashlikTilesProviderV0;
use osm::source::reqwest_source::ReqwestSource;
use pollster::FutureExt;
use std::mem;
use std::sync::{Arc, RwLock};
use wgpu::naga::compact::KeepUnused::No;
use wgpu::{
    Device, Queue, SurfaceConfiguration, SurfaceError, SurfaceTexture, Texture, TextureView,
};
use wgpu_canvas::wgpu_canvas::WgpuCanvas;

//FIXME https://github.com/gobley/gobley/issues/20
#[uniffi::export]
pub fn create_shashlik_map_api_for_ios(
    view: u64,
    metal_layer: u64,
    maximum_frames: i32,
    _tiles_db: String,
) -> ShashlikMapApi {
    panic!("Android not supported")
}

struct AndroidSurfaceAppSurface {
    app_surface: AppSurface,
    surface_texture: Option<SurfaceTexture>,
}

impl WgpuCanvas for AndroidSurfaceAppSurface {
    fn queue(&self) -> &Queue {
        &self.app_surface.queue
    }

    fn config(&self) -> &SurfaceConfiguration {
        &self.app_surface.config
    }

    fn device(&self) -> &Device {
        &self.app_surface.device
    }

    fn on_resize(&mut self) {
        self.app_surface.resize_surface();
    }

    fn create_texture_view(&mut self) -> TextureView {
        let surface_texture = self.app_surface.surface.get_current_texture().unwrap();
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
}

#[unsafe(no_mangle)]
#[jni_fn("com.shashlik.kmp.WGPUTextureView")] // TODO How to pass as a build param?
pub fn createShashlikMapApi(
    env: *mut JNIEnv<'_>,
    _: JClass,
    surface: jobject,
    emulator: jboolean,
    _tiles_db: JString,
    dpi_scale: jfloat,
) -> jlong {
    init_logger();
    let app_surface = AppSurface::new(env, surface, emulator != 0).block_on();
    let surface = AndroidSurfaceAppSurface { app_surface, surface_texture: None };
    // let mut env = unsafe { JNIEnv::from_raw(env as *mut *const _).unwrap() };
    // let tiles_db: String = env.get_string(&tiles_db).unwrap().into();
    // let tiles_sqlite_store = TilesSQLiteStore::new(tiles_db);
    let reqwest_source = ReqwestSource::new();
    let feature_processor = ShashlikFeatureProcessor::new();
    let shashlik_map = pollster::block_on(ShashlikMap::new(
        Box::new(surface),
        ShashlikTilesProviderV0::new(reqwest_source, feature_processor, dpi_scale),
    ))
    .unwrap();
    let map_api = ShashlikMapApi {
        shashlik_map: RwLock::new(shashlik_map),
    };
    Arc::into_raw(Arc::new(map_api)) as jlong
}

fn init_logger() {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Debug),
    );
    log_panics::init();
}
