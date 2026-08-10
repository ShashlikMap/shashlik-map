use crate::ShashlikMapApi;
use app_surface::AppSurface;
use app_surface::SurfaceFrame;
use jni::objects::{JClass, JObject};
use jni::objects::JString;
use jni::sys::jfloat;
use jni::sys::{jboolean, jlong, jobject};
use jni_fn::jni_fn;
use map::feature_processor::ShashlikFeatureProcessor;
use map::tiles::default_tiles_provider::DefaultTilesProvider;
use map::{DEFAULT_FONT_DATA, ShashlikMap};
use osm::source::reqwest_source::ReqwestSource;
use osm::tiles::TileStore;
use pollster::FutureExt;
use renderer_common::feature_layer_tags;
use renderer_gpu::GpuRenderer;
use renderer_gpu::wgpu_canvas::WgpuCanvas;
use std::mem;
use std::panic::catch_unwind;
use std::sync::{Arc, RwLock};
use jni::{jni_mangle, Env, EnvUnowned};
use jni::errors::ThrowRuntimeExAndDefault;
use jni::strings::JNIStr;
use log::error;
use wgpu::{CurrentSurfaceTexture,
           Device, Queue, SurfaceConfiguration, SurfaceTexture, Texture, TextureView,
};

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
            self.queue().present(surface_texture);
        }
        None
    }
}

#[jni_mangle("com.shashlik.kmp.WGPUTextureView")]// TODO How to pass as a build param?
pub fn initRustlsPlatformVerifier<'a>(
    mut unowned_env: EnvUnowned<'a>,
    _class: JClass<'a>,
    context: JObject<'a>
) {
    init_logger();
    unowned_env.with_env(|env| {
        rustls_platform_verifier::android::init_with_env(env, context)
    }).resolve::<ThrowRuntimeExAndDefault>()
}

// TODO Use jni_mangle and EnvUnowned
#[unsafe(no_mangle)]
#[jni_fn("com.shashlik.kmp.WGPUTextureView")] // TODO How to pass as a build param?
pub fn createShashlikMapApi(
    env: *mut Env<'_>,
    _: JClass,
    surface: jobject,
    emulator: jboolean,
    _tiles_db: JString,
    dpi_scale: jfloat,
) -> jlong {
    init_logger();
    let app_surface = AppSurface::new(env, surface, emulator).block_on();
    let surface = AndroidSurfaceAppSurface { app_surface, surface_texture: None };
    // let mut env = unsafe { JNIEnv::from_raw(env as *mut *const _).unwrap() };
    // let tiles_db: String = env.get_string(&tiles_db).unwrap().into();
    // let tiles_sqlite_store = TilesSQLiteStore::new(tiles_db);
    let reqwest_source = ReqwestSource::new();
    let tile_store = Box::new(TileStore::new(reqwest_source));
    let feature_processor = ShashlikFeatureProcessor::default();
    let shashlik_map = pollster::block_on(async {
        let renderer = GpuRenderer::new(feature_layer_tags(),
                                        Box::new(surface), &DEFAULT_FONT_DATA).await?;
        ShashlikMap::new(renderer,
                         DefaultTilesProvider::new(tile_store, feature_processor, dpi_scale),
        ).await
    }).unwrap();
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
