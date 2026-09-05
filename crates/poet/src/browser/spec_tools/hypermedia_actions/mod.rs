//! Hypermedia tool actions router for Poet spec tools.

mod ui;

use web_sys::{Document, Element};

/// Dispatches a hypermedia tool action to its specific handler.
pub fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    if !tool_id.starts_with("hypermedia:") {
        return None;
    }
    ui::run(document, container, tool_id)
}
