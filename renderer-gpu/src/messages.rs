use crate::canvas_api::GpuCanvasApi;
use crate::draw_commands::DrawCommands;
use std::collections::HashSet;
use renderer_common::render_group::RenderGroup;
use renderer_common::render_modifier::SpatialData;
use renderer_common::render_style::RenderStyle;
use renderer_common::style_id::StyleId;

pub(crate) enum RendererMessage {
    Draw(DrawCommands),
    ClearGroups(HashSet<String>),
}

pub enum RendererApiMsg {
    RenderGroup((String, SpatialData, Box<dyn RenderGroup<GpuCanvasApi>>)),
    UpdateStyle((StyleId, Box<dyn FnOnce(&mut RenderStyle) + Send>)),
    UpdateSpatialData((String, Box<dyn FnOnce(&mut SpatialData) + Send>)),
    ClearGroups(HashSet<String>)
}