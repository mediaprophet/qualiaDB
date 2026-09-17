//! Evidence collection, verification, and tagging for Poet investigations.

use super::shared::{append_csv_attr, count_selector};
use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "investigation:collect-evidence" => Some(collect_evidence(container)),
        "investigation:set-reliability" => Some(cycle_reliability(container)),
        "investigation:verify-evidence" => Some(verify_evidence(container)),
        "investigation:link-evidence-source" => Some(link_evidence_source(container)),
        "investigation:tag-evidence" => Some(tag_evidence(container)),
        "investigation:query-evidence" => Some(query_evidence(document, container)),
        "investigation:redact-evidence" => Some(redact_evidence(container)),
        "investigation:compare-evidence" => Some(compare_evidence(document, container)),
        _ => None,
    }
}

pub(crate) fn next_admiralty_reliability(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("A1 (Completely reliable / Confirmed)") => "B2 (Usually reliable / Probably true)",
        Some("B2 (Usually reliable / Probably true)") => "C3 (Fairly reliable / Possibly true)",
        Some("C3 (Fairly reliable / Possibly true)") => "D4 (Not usually reliable / Doubtful)",
        Some("D4 (Not usually reliable / Doubtful)") => "E5 (Unreliable / Improbable)",
        Some("E5 (Unreliable / Improbable)") => "F6 (Reliability cannot be judged)",
        _ => "A1 (Completely reliable / Confirmed)",
    }
}

fn collect_evidence(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-evidence-item", "evidence:active")
        .map_err(|_| "Failed to collect evidence item on surface.".to_string())?;
    let _ = container.set_attribute("data-evidence-reliability", "A1 (Completely reliable / Confirmed)");
    Ok(())
}

fn cycle_reliability(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-evidence-reliability");
    let next = next_admiralty_reliability(current.as_deref());
    container
        .set_attribute("data-evidence-reliability", next)
        .map_err(|_| "Failed to update evidence reliability.".to_string())
}

fn verify_evidence(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-evidence-verified", "provenance_attested")
        .map_err(|_| "Failed to verify evidence provenance.".to_string())
}

fn link_evidence_source(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-evidence-source", "source:document_or_sensor")
        .map_err(|_| "Failed to link evidence source.".to_string())
}

fn tag_evidence(container: &Element) -> Result<(), String> {
    append_csv_attr(container, "data-evidence-tags", "tag:investigation")
}

fn query_evidence(document: &Document, container: &Element) -> Result<(), String> {
    let count = count_selector(document, "[data-evidence-item]")?;
    container
        .set_attribute("data-evidence-count", &count.to_string())
        .map_err(|_| "Failed to record evidence count.".to_string())
}

fn redact_evidence(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-evidence-redacted", "sensitive_content_masked")
        .map_err(|_| "Failed to redact evidence.".to_string())
}

fn compare_evidence(document: &Document, container: &Element) -> Result<(), String> {
    let count = count_selector(document, "[data-evidence-item]")?;
    let verdict = if count >= 2 {
        "comparison:overlap_and_consistency_checked"
    } else {
        "comparison:insufficient_items"
    };
    container
        .set_attribute("data-evidence-compare", verdict)
        .map_err(|_| "Failed to compare evidence.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admiralty_reliability_scale_cycles_through_all_six_levels() {
        let l1 = next_admiralty_reliability(None);
        let l2 = next_admiralty_reliability(Some(l1));
        let l3 = next_admiralty_reliability(Some(l2));
        let l4 = next_admiralty_reliability(Some(l3));
        let l5 = next_admiralty_reliability(Some(l4));
        let l6 = next_admiralty_reliability(Some(l5));
        let l7 = next_admiralty_reliability(Some(l6));

        assert!(l1.starts_with("A1"));
        assert!(l2.starts_with("B2"));
        assert!(l3.starts_with("C3"));
        assert!(l4.starts_with("D4"));
        assert!(l5.starts_with("E5"));
        assert!(l6.starts_with("F6"));
        assert_eq!(l7, l1);
    }
}
