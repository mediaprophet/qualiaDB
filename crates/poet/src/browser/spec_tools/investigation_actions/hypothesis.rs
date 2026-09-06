//! Hypothesis proposal, evaluation, and ranking for Poet investigations.

use super::shared::{append_csv_attr, append_semicolon_attr, count_selector};
use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "investigation:propose-hypothesis" => Some(propose_hypothesis(container)),
        "investigation:set-hypothesis-status" => Some(cycle_hypothesis_status(container)),
        "investigation:evaluate-evidence" => Some(evaluate_evidence(container)),
        "investigation:set-mutually-exclusive" => Some(set_mutually_exclusive(container)),
        "investigation:query-hypotheses" => Some(query_hypotheses(document, container)),
        "investigation:hypothesis-summary" => Some(hypothesis_summary(container)),
        "investigation:rank-hypotheses" => Some(rank_hypotheses(container)),
        _ => None,
    }
}

pub(crate) fn next_hypothesis_status(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("proposed") => "testing",
        Some("testing") => "supported",
        Some("supported") => "confirmed",
        Some("confirmed") => "contradicted",
        Some("contradicted") => "disproven",
        Some("disproven") => "inconclusive",
        _ => "proposed",
    }
}

pub(crate) fn next_evaluation_verdict(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("supports") => "contradicts",
        Some("contradicts") => "neutral",
        Some("neutral") => "conditional",
        _ => "supports",
    }
}

fn propose_hypothesis(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-hypothesis-id", "H:active")
        .map_err(|_| "Failed to propose hypothesis.".to_string())?;
    let _ = container.set_attribute("data-hypothesis-status", "proposed");
    append_csv_attr(container, "data-hypotheses", "H:active")
}

fn cycle_hypothesis_status(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-hypothesis-status");
    let next = next_hypothesis_status(current.as_deref());
    container
        .set_attribute("data-hypothesis-status", next)
        .map_err(|_| "Failed to update hypothesis status.".to_string())
}

fn evaluate_evidence(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-hypothesis-evaluation");
    let verdict = next_evaluation_verdict(current.as_deref());
    container
        .set_attribute("data-hypothesis-evaluation", verdict)
        .map_err(|_| "Failed to evaluate evidence against hypothesis.".to_string())
}

fn set_mutually_exclusive(container: &Element) -> Result<(), String> {
    append_semicolon_attr(container, "data-hypothesis-exclusive", "H1|H2:mutually_exclusive")
}

fn query_hypotheses(document: &Document, container: &Element) -> Result<(), String> {
    let count = count_selector(document, "[data-hypothesis-id]")?;
    container
        .set_attribute("data-hypothesis-count", &count.to_string())
        .map_err(|_| "Failed to record hypothesis count.".to_string())
}

fn hypothesis_summary(container: &Element) -> Result<(), String> {
    let status = container
        .get_attribute("data-hypothesis-status")
        .unwrap_or_else(|| "proposed".to_string());
    let evaluation = container
        .get_attribute("data-hypothesis-evaluation")
        .unwrap_or_else(|| "none".to_string());
    let summary = format!("status={status};evaluation={evaluation};confidence=derived_from_evidence");
    container
        .set_attribute("data-hypothesis-summary", &summary)
        .map_err(|_| "Failed to summarise hypothesis.".to_string())
}

fn rank_hypotheses(container: &Element) -> Result<(), String> {
    let list = container
        .get_attribute("data-hypotheses")
        .unwrap_or_else(|| "H:active".to_string());
    container
        .set_attribute("data-hypothesis-rank", &format!("ranked:{list}"))
        .map_err(|_| "Failed to rank hypotheses.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hypothesis_status_cycles() {
        assert_eq!(next_hypothesis_status(None), "proposed");
        assert_eq!(next_hypothesis_status(Some("proposed")), "testing");
        assert_eq!(next_hypothesis_status(Some("inconclusive")), "proposed");
    }

    #[test]
    fn evaluation_verdict_cycles() {
        assert_eq!(next_evaluation_verdict(None), "supports");
        assert_eq!(next_evaluation_verdict(Some("conditional")), "supports");
    }
}
