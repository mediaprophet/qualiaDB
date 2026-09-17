//! AI and NLP tool actions router for Poet spec tools.

mod graph_bridge;
mod neural;
mod symbolic;

use web_sys::{Document, Element};

/// Dispatches an AI tool action to its specific handler.
pub fn run(_document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    if !tool_id.starts_with("ai:") {
        return None;
    }
    symbolic::run(container, tool_id)
        .or_else(|| neural::run(container, tool_id))
        .or_else(|| graph_bridge::run(container, tool_id))
}
