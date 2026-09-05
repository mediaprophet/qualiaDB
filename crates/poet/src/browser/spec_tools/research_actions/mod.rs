//! Research tool actions router for Poet spec tools.

mod corpus;
mod enquiry;

use web_sys::{Document, Element};

/// Dispatches a research tool action to its specific handler.
pub fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    if !tool_id.starts_with("research:") {
        return None;
    }
    enquiry::run(document, container, tool_id)
        .or_else(|| corpus::run(container, tool_id))
}
