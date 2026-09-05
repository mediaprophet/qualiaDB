//! Audio tool actions router for Poet spec tools.

mod mixing;
mod tracks;

use web_sys::{Document, Element};

/// Dispatches an audio tool action to its specific handler.
pub fn run(_document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    if !tool_id.starts_with("audio:") {
        return None;
    }
    tracks::run(container, tool_id)
        .or_else(|| mixing::run(container, tool_id))
}
