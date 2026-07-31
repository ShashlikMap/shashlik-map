use crate::render_group::RenderGroup;
use crate::render_modifier::SpatialData;
use crate::render_style::RenderStyle;
use crate::style_id::StyleId;
use crate::{CanvasApi, RendererApi};
use std::collections::HashSet;
use std::sync::mpsc::Sender;

pub enum RendererApiMsg<T: CanvasApi> {
    RenderGroup(String, SpatialData, Box<dyn RenderGroup<T>>),
    UpdateStyle(StyleId, Box<dyn FnOnce(&mut RenderStyle) + Send>),
    UpdateSpatialData(String, Box<dyn FnOnce(&mut SpatialData) + Send>),
    ClearGroups(HashSet<String>),
}

pub struct CommonRendererApi<T: CanvasApi> {
    sender: Sender<RendererApiMsg<T>>,
}

impl<T: CanvasApi> CommonRendererApi<T> {
    pub fn new(sender: Sender<RendererApiMsg<T>>) -> CommonRendererApi<T> {
        Self { sender }
    }
}

impl<T: CanvasApi> RendererApi for CommonRendererApi<T> {
    type CANVAS = T;

    fn add_render_group(
        &self,
        key: String,
        spatial_data: SpatialData,
        group: Box<dyn RenderGroup<Self::CANVAS>>,
    ) {
        self.sender
            .send(RendererApiMsg::RenderGroup(key, spatial_data, group))
            .unwrap();
    }

    fn clear_render_groups(&self, keys: HashSet<String>) {
        self.sender
            .send(RendererApiMsg::ClearGroups(keys))
            .unwrap();
    }

    fn update_style<F: FnOnce(&mut RenderStyle) + Send + 'static>(
        &self,
        style_id: StyleId,
        updater: F,
    ) {
        self.sender
            .send(RendererApiMsg::UpdateStyle(style_id, Box::new(updater)))
            .unwrap();
    }

    fn update_spatial_data<F: FnOnce(&mut SpatialData) + Send + 'static>(
        &self,
        key: String,
        updater: F,
    ) {
        self.sender
            .send(RendererApiMsg::UpdateSpatialData(key, Box::new(updater)))
            .unwrap();
    }
}
