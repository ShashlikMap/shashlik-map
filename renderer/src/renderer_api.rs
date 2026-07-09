use crate::canvas_api::GpuCanvasApi;
use crate::messages::RendererApiMsg;
use std::collections::HashSet;
use std::sync::mpsc::Sender;
use wgpu_canvas::render_group::RenderGroup;
use wgpu_canvas::render_modifier::SpatialData;
use wgpu_canvas::render_style::RenderStyle;
use wgpu_canvas::style_id::StyleId;
use wgpu_canvas::RendererApi;

pub struct GpuRendererApi {
    renderer_api_tx: Sender<RendererApiMsg>,
}

impl RendererApi for GpuRendererApi {
    type CANVAS = GpuCanvasApi;

    fn add_render_group(&self, key: String, spatial_data: SpatialData, group: Box<dyn RenderGroup<GpuCanvasApi>>) {
        self.renderer_api_tx
            .send(RendererApiMsg::RenderGroup((key, spatial_data, group)))
            .expect("RendererApi add_render_group sender failed.");
    }

    fn clear_render_groups(&self, keys: HashSet<String>) {
        self.renderer_api_tx
            .send(RendererApiMsg::ClearGroups(keys))
            .expect("RendererApi clear_render_groups sender failed.");
    }

    fn update_style<F: FnOnce(&mut RenderStyle) + Send + 'static>(
        &self,
        style_id: StyleId,
        updater: F,
    ) {
        self.renderer_api_tx
            .send(RendererApiMsg::UpdateStyle((style_id, Box::new(updater))))
            .expect("RendererApi update_style sender failed.");
    }

    fn update_spatial_data<F: FnOnce(&mut SpatialData) + Send + 'static>(
        &self,
        key: String,
        updater: F,
    ) {
        self.renderer_api_tx
            .send(RendererApiMsg::UpdateSpatialData((key, Box::new(updater))))
            .expect("RendererApi update_spatial_data sender failed.");
    }
}

impl GpuRendererApi {
    pub fn new(renderer_api_tx: Sender<RendererApiMsg>) -> Self {
        Self { renderer_api_tx }
    }
}
