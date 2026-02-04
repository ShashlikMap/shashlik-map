use map::feature_processor::ShashlikFeatureProcessor;
use map::tiles::shashlik_tiles_provider_v0::ShashlikTilesProviderV0;
use map::ShashlikMap;
use osm::source::reqwest_source::ReqwestSource;
use slint::wgpu_28::{WGPUConfiguration, WGPUSettings};
use slint::{GraphicsAPI, RenderingState};
use wgpu::{Device, Limits, Queue, SurfaceConfiguration, SurfaceError, SurfaceTexture, Texture, TextureFormat, TextureUsages};
use wgpu_canvas::wgpu_canvas::WgpuCanvas;

slint::include_modules!();

struct SlintWgpuCanvas(Queue, Device, SurfaceConfiguration, Texture);

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

    fn get_current_texture(&self) -> Result<SurfaceTexture, SurfaceError> {
        todo!()
    }

    fn get_current_texture2(&self) -> &Texture {
        &self.3
    }

    fn on_resize(&mut self) {
    }

    fn on_pre_render(&self) {
    }

    fn on_post_render(&self) {
    }
}
fn main() {
    env_logger::init();

    // let (sender, receiver) = mpsc::channel();

    // let app = App::new(
    //     Box::new(|| ShashlikTilesProviderV0::new(ReqwestSource::new(), ShashlikFeatureProcessor::new(), 1.0)),
    //     receiver,
    // );
    // let event_loop = EventLoop::with_user_event();

    // slint::platform::set_platform(Box::new(
    //     i_slint_backend_winit::Backend::builder()
    //         .with_event_loop_builder(event_loop)
    //         .with_custom_application_handler(Box::new(app))
    //         .build()
    //         .unwrap(),
    // ))
    // .unwrap();

    let mut wgpu_settings = WGPUSettings::default();
    println!("wgpu_settings = {:?}",wgpu_settings.backends);
    wgpu_settings.device_required_limits = Limits::downlevel_defaults();

    slint::BackendSelector::new()
        .require_wgpu_28(WGPUConfiguration::Automatic(wgpu_settings))
        .select()
        .expect("Unable to create Slint backend with WGPU based renderer");

    let mut shashlik_map = None;


    let ui = AppKiol::new().unwrap();
    let ui_weak = ui.as_weak();

    ui.window().set_rendering_notifier(move |state,graphics_api: &GraphicsAPI| {
        match state {
            RenderingState::RenderingSetup => {
                match graphics_api {
                    GraphicsAPI::WGPU28 { instance, device,queue , .. } => {
                        let ttt= device.create_texture(&wgpu::TextureDescriptor {
                            label: None,
                            size: wgpu::Extent3d { width: 1600, height: 1200, depth_or_array_layers: 1 },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                            view_formats: &[],
                        });
                        let config = SurfaceConfiguration {
                            usage: TextureUsages::RENDER_ATTACHMENT,
                            format: TextureFormat::Rgba8UnormSrgb,
                            width: 1600,
                            height: 1200,
                            present_mode: Default::default(),
                            desired_maximum_frame_latency: 0,
                            alpha_mode: Default::default(),
                            view_formats: vec![],
                        };
                        let canvas = SlintWgpuCanvas(queue.clone(), device.clone(), config, ttt);
                        let tiles_provider = ShashlikTilesProviderV0::new(ReqwestSource::new(), ShashlikFeatureProcessor::new(), 1.0);
                        let hh = pollster::block_on(ShashlikMap::new(
                            Box::new(canvas),
                            tiles_provider
                        ));
                        shashlik_map = Some(hh.unwrap());
                        shashlik_map.as_mut().unwrap().resize(1600, 1200);

                        // renderer = Some(DemoRenderer::new(device, queue));
                    }
                    _ => {}
                }
            }
            RenderingState::BeforeRendering => {
                if let (Some(shashlik_map), Some(app)) = (shashlik_map.as_mut(), ui_weak.upgrade()) {
                   let target_texture = shashlik_map.update_and_render();
                    app.set_texture(slint::Image::try_from(target_texture).unwrap());
                    app.window().request_redraw();
                }
            }
            RenderingState::AfterRendering => {}
            RenderingState::RenderingTeardown => {},
            _ => panic!("Unhandled RenderingState ")
        }
    }).expect("KIOL1");

    // let sender_clone = sender.clone();
    // ui.on_open_kml_button_click(move || {
    //     let path = DialogBuilder::file()
    //         .set_location("~/Desktop")
    //         .add_filter("KML", ["kml"])
    //         .open_single_file()
    //         .show()
    //         .unwrap();
    //     if let Some(path) = path {
    //         sender_clone.send(CustomUIEvent::KMLPath(path)).unwrap();
    //     }
    // });

    ui.run().unwrap();
}
