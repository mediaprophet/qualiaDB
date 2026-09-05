//! Spatial 3D tool actions router for Poet spec tools.

mod mesh;
mod viewport;

use web_sys::{Document, Element};

/// Dispatches a spatial 3D tool action to its specific handler.
pub fn run(_document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    if !tool_id.starts_with("spatial3d:") {
        return None;
    }
    mesh::run(container, tool_id)
        .or_else(|| viewport::run(container, tool_id))
}
