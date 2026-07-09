use geo_types::Coord;
use glam::DVec2;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tiny_skia::{Paint, Pixmap, Rect, Transform};
use renderer_common::geometry_data::GeometryData;
use renderer_common::render_group::RenderGroup;
use renderer_common::render_modifier::SpatialData;
use renderer_common::render_style::RenderStyle;
use renderer_common::style_id::StyleId;
use renderer_common::{CanvasApi, RendererApi, Renderer, RendererUpdateData};

/// This is the very beginning of CPU renderer-gpu. So far, just a stub animation

pub struct CpuRenderer {
    start_time: Instant,
    cpu_renderer_api: Arc<CpuRendererApi>,
}
pub struct CpuRendererApi {}
pub struct CpuCanvasApi {}

impl CanvasApi for CpuCanvasApi {
    fn set_feature_layer_tag(&mut self, tag: Option<String>) {
    }

    fn geometry_data(&mut self, geometry_data: GeometryData) {
    }
}

impl RendererApi for CpuRendererApi {
    type CANVAS = CpuCanvasApi;

    fn add_render_group(
        &self,
        key: String,
        spatial_data: SpatialData,
        group: Box<dyn RenderGroup<Self::CANVAS>>,
    ) {
    }

    fn clear_render_groups(&self, keys: HashSet<String>) {}

    fn update_style<F: FnOnce(&mut RenderStyle) + Send + 'static>(
        &self,
        style_id: StyleId,
        updater: F,
    ) {
    }

    fn update_spatial_data<F: FnOnce(&mut SpatialData) + Send + 'static>(
        &self,
        key: String,
        updater: F,
    ) {
    }
}

impl CpuRenderer {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            cpu_renderer_api: Arc::new(CpuRendererApi {}),
        }
    }
}

impl Renderer for CpuRenderer {
    type RAPI = CpuRendererApi;
    type OUTPUT = Pixmap;

    fn screen_size(&self) -> (f32, f32) {
        (400.0, 400.0)
    }

    fn resize(&mut self, width: u32, height: u32) {}

    fn update(&mut self, data: RendererUpdateData) {}

    fn clip_to_world(&self, coord: &Coord) -> Option<DVec2> {
        Some(DVec2::splat(0.0))
    }

    fn render(&mut self) -> Option<Self::OUTPUT> {
        const WIDTH: u32 = 400;
        const HEIGHT: u32 = 400;
        let mut pixmap = Pixmap::new(WIDTH, HEIGHT).unwrap();
        pixmap.fill(tiny_skia::Color::from_rgba8(30, 30, 30, 255));

        let time_elapsed = self.start_time.elapsed().as_secs_f32();
        let x_offset = (time_elapsed.sin() * 100.0) + 150.0;

        let mut paint = Paint::default();
        paint.set_color(tiny_skia::Color::from_rgba8(46, 204, 113, 255)); // Green
        paint.anti_alias = true;

        if let Some(rect) = Rect::from_xywh(x_offset, 150.0, 100.0, 100.0) {
            pixmap.fill_rect(rect, &paint, Transform::identity(), None);
        }

        Some(pixmap)
    }

    fn api(&self) -> Arc<CpuRendererApi> {
        self.cpu_renderer_api.clone()
    }
}
