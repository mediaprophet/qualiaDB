//! Theatrical production tool actions router for Poet spec tools.

mod cues;
mod dmx;

use web_sys::{Document, Element};

/// Dispatches a production tool action to its specific handler.
pub fn run(_document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    if !tool_id.starts_with("productions:") {
        return None;
    }
    dmx::run(container, tool_id)
        .or_else(|| cues::run(container, tool_id))
}
