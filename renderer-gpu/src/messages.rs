use crate::draw_commands::DrawCommands;
use std::collections::HashSet;

pub(crate) enum RendererMessage {
    Draw(DrawCommands),
    ClearGroups(HashSet<String>),
    BringToFront(String, Option<String>),
    LinearRefs(Vec<f32>),
}