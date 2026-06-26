use crate::draw_commands::{DrawCommands, DrawCommands2};
use std::collections::HashSet;
use crate::modifier::render_modifier::SpatialData;
use crate::render_group::RenderGroup;
use crate::styles::render_style::RenderStyle;
use crate::styles::style_id::StyleId;

pub(crate) enum RendererMessage {
    Draw(DrawCommands),
    DrawShapes(DrawCommands2),
    ClearGroups(HashSet<String>),
}

pub enum RendererApiMsg {
    RenderGroup((String, SpatialData, Box<dyn RenderGroup>)),
    UpdateStyle((StyleId, Box<dyn FnOnce(&mut RenderStyle) + Send>)),
    UpdateSpatialData((String, Box<dyn FnOnce(&mut SpatialData) + Send>)),
    ClearGroups(HashSet<String>)
}