//! Hypermedia tool actions router for Poet spec tools.

mod interactive;
mod packaging;
mod sync;
mod ui;

use web_sys::{Document, Element};

/// Dispatches a hypermedia tool action to its specific handler.
pub fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    if !tool_id.starts_with("hypermedia:") {
        return None;
    }
    interactive::run(document, container, tool_id)
        .or_else(|| packaging::run(container, tool_id))
        .or_else(|| sync::run(container, tool_id))
        .or_else(|| ui::run(document, container, tool_id))
}
