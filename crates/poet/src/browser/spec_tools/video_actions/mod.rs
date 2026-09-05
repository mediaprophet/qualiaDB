//! Video tool actions router for Poet spec tools.

mod effects;
mod timeline;

use web_sys::{Document, Element};

/// Dispatches a video tool action to its specific handler.
pub fn run(_document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    if !tool_id.starts_with("video:") {
        return None;
    }
    timeline::run(container, tool_id)
        .or_else(|| effects::run(container, tool_id))
}
