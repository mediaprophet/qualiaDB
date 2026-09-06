//! Hidden link inference and provenance gap detection for Poet research.

use super::util::{append_csv_attr, append_nested, count_document, count_within, next_confidence};
use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "research:infer-dark-link" => Some(infer_dark_link(document, container)),
        "research:detect-provenance-gaps" => Some(detect_provenance_gaps(container)),
        "research:detect-concealment-patterns" => Some(detect_concealment_patterns(container)),
        "research:link-dark-link-evidence" => Some(link_dark_link_evidence(container)),
        "research:set-dark-link-confidence" => Some(set_dark_link_confidence(container)),
        "research:query-dark-links" => Some(query_dark_links(document, container)),
        "research:confirm-dark-link" => Some(confirm_dark_link(container)),
        "research:refute-dark-link" => Some(refute_dark_link(container)),
        "research:trace-dark-link-provenance" => Some(trace_dark_link_provenance(container)),
        _ => None,
    }
}

fn infer_dark_link(document: &Document, container: &Element) -> Result<(), String> {
    append_nested(
        document,
        container,
        "span",
        "data-dark-link",
        "dark_link:inferred",
        &[
            ("data-dark-link-type", "latent"),
            ("data-dark-link-method", "pattern_inference"),
            ("data-dark-link-status", "provisional"),
        ],
    )
}

fn detect_provenance_gaps(container: &Element) -> Result<(), String> {
    let corpus = count_within(container, "[data-corpus-item]")?;
    let inferences = count_within(container, "[data-research-inference]")?;
    let gaps = if corpus == 0 && inferences > 0 {
        "gaps:missing_source_records"
    } else {
        "gaps:none_detected"
    };
    container
        .set_attribute("data-provenance-gaps", gaps)
        .map_err(|_| "Failed to detect provenance gaps.".to_string())
}

fn detect_concealment_patterns(container: &Element) -> Result<(), String> {
    let dark = count_within(container, "[data-dark-link]")?;
    let pattern = if dark > 0 {
        "concealment:selective_omission_suspected"
    } else {
        "concealment:no_pattern_detected"
    };
    container
        .set_attribute("data-concealment-patterns", pattern)
        .map_err(|_| "Failed to detect concealment patterns.".to_string())
}

fn link_dark_link_evidence(container: &Element) -> Result<(), String> {
    append_csv_attr(
        container,
        "data-dark-link-evidence",
        "evidence:indirect_corpus_support",
    )
}

fn set_dark_link_confidence(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-dark-link-confidence");
    let next = next_confidence(current.as_deref());
    container
        .set_attribute("data-dark-link-confidence", next)
        .map_err(|_| "Failed to set dark-link confidence.".to_string())
}

fn query_dark_links(document: &Document, container: &Element) -> Result<(), String> {
    let local = count_within(container, "[data-dark-link]")?;
    let global = count_document(document, "[data-dark-link]")?;
    container
        .set_attribute(
            "data-dark-link-query",
            &format!("local={local};canvas={global}"),
        )
        .map_err(|_| "Failed to query dark links.".to_string())
}

fn confirm_dark_link(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-dark-link-status", "confirmed")
        .map_err(|_| "Failed to confirm dark link.".to_string())
}

fn refute_dark_link(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-dark-link-status", "refuted")
        .map_err(|_| "Failed to refute dark link.".to_string())
}

fn trace_dark_link_provenance(container: &Element) -> Result<(), String> {
    container
        .set_attribute(
            "data-dark-link-provenance",
            "trace:evidence->inference->gap_notes",
        )
        .map_err(|_| "Failed to trace dark-link provenance.".to_string())
}
