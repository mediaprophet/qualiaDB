//! Investigation tool actions router for Poet spec tools.

mod case;
mod evidence;

use web_sys::{Document, Element};

/// Dispatches an investigation tool action to its specific handler.
pub fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    if !tool_id.starts_with("investigation:") {
        return None;
    }
    case::run(document, container, tool_id)
        .or_else(|| evidence::run(document, container, tool_id))
}
