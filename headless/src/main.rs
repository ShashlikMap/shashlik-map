use std::env;
use map::feature_processor::ShashlikFeatureProcessor;
use map::tiles::default_tiles_provider::DefaultTilesProvider;
use map::{DEFAULT_FONT_DATA, ShashlikMap};
use osm::source::reqwest_source::ReqwestSource;
use osm::tiles::TileStore;
use renderer_common::feature_layer_tags;
use renderer_gpu::GpuRenderer;
use renderer_gpu::render_config::RenderConfig;
use renderer_gpu::wgpu_canvas::DefaultWgpuCanvas;
use std::thread::sleep;
use std::time::Duration;
use geo_types::Point;
use wgpu::{
    Device, DeviceDescriptor, Features, Label, Limits, MemoryHints, PowerPreference, Queue,
};
use map::overlay::ShapeType;

fn main() {
    let args: Vec<String> = env::args().collect();
    let is_globe = args.len() > 1 && args.contains(&"-globe".to_string());
    let with_overlay = args.len() > 1 && args.contains(&"-overlay".to_string());
    println!("Headless mode started, is_globe: {}, with_overlay: {}", is_globe, with_overlay);
    let (device, queue) = pollster::block_on(async { create_wgpu().await });

    // the size is what used for macOS is this time
    let size = if !is_globe {
        wgpu::Extent3d {
            width: 1561u32,
            height: 1168u32,
            depth_or_array_layers: 1,
        }
    } else {
        wgpu::Extent3d {
            width: 1080u32,
            height: 1920u32,
            depth_or_array_layers: 1,
        }
    };
    let target_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let canvas = DefaultWgpuCanvas::new(queue.clone(), device.clone(), target_texture);
    let tiles_provider = DefaultTilesProvider::new(
        Box::new(TileStore::new(ReqwestSource::new())),
        ShashlikFeatureProcessor::default(),
        1.0,
    );

    let mut map = pollster::block_on(async {
        let mut render_config = RenderConfig::default();
        render_config.headless = true;
        let renderer = GpuRenderer::new_with_config(
            render_config,
            feature_layer_tags(),
            Box::new(canvas),
            &DEFAULT_FONT_DATA,
        )
        .await?;

        ShashlikMap::new(renderer, tiles_provider).await
    })
    .unwrap();

    map.resize(size.width, size.height);
    map.set_camera_follow_mode(!with_overlay);
    map.set_cam_follow_zoom_lock(None);
    if is_globe {
        map.zoom_delta(0.000035, (0.0, 0.0));
        map.set_lon_lat_bearing(105.757080078125, 25.69100828125, None);
    } else {
        map.zoom_delta(1.15, (0.0, 0.0));
        map.set_lon_lat_bearing(139.757080078125, 35.69100828125, None);
        if with_overlay {
            let converter = map.create_location_coord_converter();
            map.overlay().add_overlay_shape(converter, vec![Point::new(139.757080078125, 35.69100828125),
                                                            Point::new(139.757780078125, 35.69190828125),
                                                            Point::new(139.757680078125, 35.69050828125)], ShapeType::Polygon,
                                            [0.0, 0.0, 1.0]);
            let converter = map.create_location_coord_converter();
            map.overlay().add_overlay_shape(converter, vec![Point::new(139.757080078125, 35.69100828125),
                                                            Point::new(139.757780078125, 35.69190828125)], ShapeType::Line(None),
                                            [1.0, 0.0, 0.0]);
        }
    }

    println!("Headless mode. Run frames");
    let frames_to_run = if is_globe { 300 } else { 120 };
    sleep(Duration::from_secs(1));
    map.update_and_render(());
    for _ in 0..frames_to_run {
        sleep(Duration::from_millis(16));
        map.update_and_render(());
    }
    println!("Headless mode completed");
}

async fn create_wgpu() -> (Device, Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    // back to software renderer
    let force_fallback_adapter = cfg!(not(target_os = "macos"));
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter,
            apply_limit_buckets: true,
        })
        .await
        .unwrap();

    let mut device_descriptor = DeviceDescriptor {
        label: Label::from("HeadlessDeviceDescriptor"),
        required_features: Features::CLEAR_TEXTURE | Features::IMMEDIATES,
        required_limits: Limits::downlevel_defaults(),
        experimental_features: Default::default(),
        memory_hints: MemoryHints::MemoryUsage,
        trace: Default::default(),
    };
    device_descriptor.required_limits.max_immediate_size = 4;
    adapter.request_device(&device_descriptor).await.unwrap()
}
