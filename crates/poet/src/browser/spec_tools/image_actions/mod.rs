//! Image tool actions router for Poet spec tools.

mod brushes;
mod layers;
mod vector;

use web_sys::{Document, Element};

/// Dispatches an image tool action to its specific handler.
pub fn run(_document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    if !tool_id.starts_with("image:") {
        return None;
    }
    layers::run(container, tool_id)
        .or_else(|| brushes::run(container, tool_id))
        .or_else(|| vector::run(container, tool_id))
}
