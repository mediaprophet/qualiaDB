//! Investigation tool actions router for Poet spec tools.

mod case;
mod causal;
mod custody;
mod evidence;
mod graph;
mod hyp_graph;
mod hypothesis;
mod links;
mod risk;
mod scenario;
mod shared;
mod timeline;
mod trend;

use web_sys::{Document, Element};

/// Dispatches an investigation tool action to its specific handler.
pub fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    if !tool_id.starts_with("investigation:") {
        return None;
    }
    case::run(document, container, tool_id)
        .or_else(|| evidence::run(document, container, tool_id))
        .or_else(|| custody::run(document, container, tool_id))
        .or_else(|| hypothesis::run(document, container, tool_id))
        .or_else(|| hyp_graph::run(document, container, tool_id))
        .or_else(|| timeline::run(document, container, tool_id))
        .or_else(|| graph::run(document, container, tool_id))
        .or_else(|| links::run(document, container, tool_id))
        .or_else(|| scenario::run(document, container, tool_id))
        .or_else(|| trend::run(document, container, tool_id))
        .or_else(|| risk::run(document, container, tool_id))
        .or_else(|| causal::run(document, container, tool_id))
}
