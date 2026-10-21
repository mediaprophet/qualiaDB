//! Project-grounded local LLM and specialist-agent console.

use web_sys::{Document, Element};

/// Builds agent-console-view.
pub fn build_agent_console_view(document: &Document) -> Element {
    super::agent_console_workspace::build_agent_console_view(document)
}
