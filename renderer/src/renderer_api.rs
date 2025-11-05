use std::collections::HashSet;
use std::sync::mpsc::Sender;
use crate::messages::RendererApiMsg;
use crate::modifier::render_modifier::SpatialData;
use crate::render_group::RenderGroup;
use crate::styles::render_style::RenderStyle;
use crate::styles::style_id::StyleId;

pub struct RendererApi {
    renderer_api_tx: Sender<RendererApiMsg>,
}

impl RendererApi {
    pub fn new(
        renderer_api_tx: Sender<RendererApiMsg>,
    ) -> Self {
        Self {
            renderer_api_tx,
        }
    }
    pub fn add_render_group(
        &self,
        key: String,
        layer: usize,
        spatial_data: SpatialData,
        group: Box<dyn RenderGroup>,
    ) {
        self.renderer_api_tx
            .send(RendererApiMsg::RenderGroup((key, layer, spatial_data, group)))
            .expect("RendererApi add_render_group sender failed.");
    }

    pub fn clear_render_groups(&self, keys: HashSet<String>) {
        self.renderer_api_tx
            .send(RendererApiMsg::ClearGroups(keys))
            .expect("RendererApi clear_render_groups sender failed.");
    }

    pub fn update_style<F: FnOnce(&mut RenderStyle) + Send + 'static>(
        &self,
        style_id: StyleId,
        updater: F,
    ) {
        self.renderer_api_tx
            .send(RendererApiMsg::UpdateStyle((
                style_id,
                Box::new(updater),
            )))
            .expect("RendererApi update_style sender failed.");
    }

    pub fn update_spatial_data<F: FnOnce(&mut SpatialData) + Send + 'static>(
        &self,
        key: String,
        updater: F,
    ) {
        self.renderer_api_tx
            .send(RendererApiMsg::UpdateSpatialData((key, Box::new(updater))))
            .expect("RendererApi update_spatial_data sender failed.");
    }
}