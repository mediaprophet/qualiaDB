//! Epistemic tool actions router for Poet spec tools.

mod assessments;
mod context;

use web_sys::{Document, Element};

/// Dispatches an epistemic tool action to its specific handler.
pub fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    if !tool_id.starts_with("epistemic:") {
        return None;
    }
    assessments::run(document, container, tool_id)
        .or_else(|| context::run(document, container, tool_id))
}
