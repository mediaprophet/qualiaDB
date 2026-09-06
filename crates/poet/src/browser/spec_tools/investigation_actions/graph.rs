//! Graph tool router — link analysis and hypothesis graph chains.

use super::{hyp_graph, links};
use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    links::run(document, container, tool_id).or_else(|| hyp_graph::run(document, container, tool_id))
}
