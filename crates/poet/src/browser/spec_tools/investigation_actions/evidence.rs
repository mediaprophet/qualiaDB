//! Evidence collection, verification, and hypothesis testing for Poet investigations.

use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "investigation:collect-evidence" => Some(collect_evidence(container)),
        "investigation:set-reliability" => Some(cycle_reliability(container)),
        "investigation:verify-evidence" => Some(verify_evidence(container)),
        "investigation:add-chain-of-custody" => Some(add_chain_of_custody(container)),
        "investigation:corroborate-evidence" => Some(corroborate_evidence(container)),
        "investigation:challenge-evidence" => Some(challenge_evidence(container)),
        "investigation:query-evidence" => Some(query_evidence(document, container)),
        "investigation:add-hypothesis" => Some(add_hypothesis(container)),
        "investigation:test-hypothesis" => Some(test_hypothesis(container)),
        "investigation:link-evidence-hypothesis" => Some(link_evidence_hypothesis(container)),
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

fn add_chain_of_custody(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-chain-of-custody")
        .unwrap_or_default();
    let entry = "custody:received_and_hashed";
    let updated = if current.is_empty() {
        entry.to_string()
    } else {
        format!("{current};{entry}")
    };
    container
        .set_attribute("data-chain-of-custody", &updated)
        .map_err(|_| "Failed to append chain of custody.".to_string())
}

fn corroborate_evidence(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-evidence-corroboration", "corroborated_by_independent_source")
        .map_err(|_| "Failed to corroborate evidence.".to_string())
}

fn challenge_evidence(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-evidence-challenge", "formal_rebuttal_registered")
        .map_err(|_| "Failed to register evidence challenge.".to_string())
}

fn query_evidence(document: &Document, container: &Element) -> Result<(), String> {
    let items = document
        .query_selector_all("[data-evidence-item]")
        .map_err(|_| "Failed to query evidence items.".to_string())?;
    container
        .set_attribute("data-evidence-count", &items.length().to_string())
        .map_err(|_| "Failed to record evidence count.".to_string())
}

fn add_hypothesis(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-case-hypotheses")
        .unwrap_or_default();
    let entry = "H:explanatory_model";
    let updated = if current.is_empty() {
        entry.to_string()
    } else {
        format!("{current};{entry}")
    };
    container
        .set_attribute("data-case-hypotheses", &updated)
        .map_err(|_| "Failed to add investigation hypothesis.".to_string())
}

fn test_hypothesis(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-hypothesis-tested", "consistency_checked_against_evidence")
        .map_err(|_| "Failed to test hypothesis.".to_string())
}

fn link_evidence_hypothesis(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-evidence-hypothesis-link", "linked:H1_supports")
        .map_err(|_| "Failed to link evidence to hypothesis.".to_string())
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
