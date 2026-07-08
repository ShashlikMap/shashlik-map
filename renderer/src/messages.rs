use crate::canvas_api::CanvasApi;
use crate::draw_commands::DrawCommands;
use std::collections::HashSet;
use wgpu_canvas::render_group::RenderGroup;
use wgpu_canvas::render_modifier::SpatialData;
use wgpu_canvas::render_style::RenderStyle;
use wgpu_canvas::style_id::StyleId;

pub(crate) enum RendererMessage {
    Draw(DrawCommands),
    ClearGroups(HashSet<String>),
}

pub enum RendererApiMsg {
    RenderGroup((String, SpatialData, Box<dyn RenderGroup<CanvasApi>>)),
    UpdateStyle((StyleId, Box<dyn FnOnce(&mut RenderStyle) + Send>)),
    UpdateSpatialData((String, Box<dyn FnOnce(&mut SpatialData) + Send>)),
    ClearGroups(HashSet<String>)
}