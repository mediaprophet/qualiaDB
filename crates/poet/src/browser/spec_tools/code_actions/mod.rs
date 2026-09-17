//! Code tool actions router for Poet spec tools.

mod quin;
mod vibe;

use web_sys::{Document, Element};

/// Dispatches a code tool action to its specific handler.
pub fn run(_document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    if !tool_id.starts_with("code:") {
        return None;
    }
    vibe::run(container, tool_id)
        .or_else(|| quin::run(container, tool_id))
}
