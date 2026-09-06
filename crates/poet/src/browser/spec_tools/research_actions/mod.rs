//! Research tool actions router for Poet spec tools.

mod bootstrap;
mod corpus;
mod dark_links;
mod dynamics;
mod enquiry;
mod inference;
mod provenance;
mod synthesis;
mod util;

use web_sys::{Document, Element};

/// Dispatches a research tool action to its specific handler.
pub fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    if !tool_id.starts_with("research:") {
        return None;
    }
    enquiry::run(document, container, tool_id)
        .or_else(|| corpus::run(document, container, tool_id))
        .or_else(|| dynamics::run(document, container, tool_id))
        .or_else(|| dark_links::run(document, container, tool_id))
        .or_else(|| inference::run(document, container, tool_id))
        .or_else(|| synthesis::run(document, container, tool_id))
        .or_else(|| provenance::run(document, container, tool_id))
        .or_else(|| bootstrap::run(document, container, tool_id))
}
