use crate::draw_commands::DrawCommands;
use std::collections::HashSet;

pub(crate) enum RendererMessage {
    Draw(DrawCommands),
    ClearGroups(HashSet<String>),
}