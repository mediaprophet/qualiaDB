//! Local grounding query and comparison microformats (assess/verify are Live).

use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "epistemic:query-grounding" => Some(query_grounding(document, container)),
        "epistemic:compare-grounding" => Some(compare_grounding(document, container)),
        _ => None,
    }
}

fn query_grounding(document: &Document, container: &Element) -> Result<(), String> {
    let list = document
        .query_selector_all("[data-grounding-type]")
        .map_err(|_| "Failed to query grounding notes.".to_string())?;
    container
        .set_attribute("data-grounding-count", &list.length().to_string())
        .map_err(|_| "Failed to record grounding count.".to_string())
}

fn compare_grounding(document: &Document, container: &Element) -> Result<(), String> {
    let list = document
        .query_selector_all("[data-grounding-type]")
        .map_err(|_| "Failed to query grounding notes for comparison.".to_string())?;
    let comparison = if list.length() >= 2 {
        "multi-agent grounding comparison active"
    } else {
        "single grounding note on canvas"
    };
    container
        .set_attribute("data-grounding-comparison", comparison)
        .map_err(|_| "Failed to set grounding comparison.".to_string())
}
