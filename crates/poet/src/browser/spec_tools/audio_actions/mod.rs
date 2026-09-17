//! Audio tool actions router for Poet spec tools.

mod editing;
mod effects;
mod inspect;
mod midi;
mod mixing;
mod synthesis;
mod tracks;
mod transport;

use web_sys::{Document, Element};

/// Dispatches an audio tool action to its specific handler.
pub fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    if !tool_id.starts_with("audio:") {
        return None;
    }
    transport::run(container, tool_id)
        .or_else(|| tracks::run(document, container, tool_id))
        .or_else(|| mixing::run(container, tool_id))
        .or_else(|| editing::run(container, tool_id))
        .or_else(|| midi::run(container, tool_id))
        .or_else(|| effects::run(container, tool_id))
        .or_else(|| synthesis::run(container, tool_id))
        .or_else(|| inspect::run(container, tool_id))
}
