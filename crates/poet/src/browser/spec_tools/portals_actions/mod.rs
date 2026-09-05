//! Portals tool actions router for Poet spec tools.

mod world;

use web_sys::{Document, Element};

/// Dispatches a portals tool action to its specific handler.
pub fn run(_document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    if !tool_id.starts_with("portals:") {
        return None;
    }
    world::run(container, tool_id)
}
