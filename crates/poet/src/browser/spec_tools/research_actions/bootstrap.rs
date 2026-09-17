//! Bootstrap, frontier, and enrichment workflows for Poet research.

use super::util::{append_csv_attr, append_nested, count_within};
use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "research:assess-deficit" => Some(assess_deficit(container)),
        "research:identify-frontier" => Some(identify_frontier(container)),
        "research:generate-bootstrap-hypotheses" => {
            Some(generate_bootstrap_hypotheses(document, container))
        }
        "research:refine-bootstrap" => Some(refine_bootstrap(container)),
        "research:confirm-bootstrap" => Some(set_bootstrap_status(container, "confirmed")),
        "research:reframe-bootstrap" => Some(reframe_bootstrap(container)),
        "research:disprove-bootstrap" => Some(set_bootstrap_status(container, "disproved")),
        "research:supersede-bootstrap" => Some(supersede_bootstrap(container)),
        "research:promote-bootstrap" => Some(promote_bootstrap(container)),
        "research:record-enrichment" => Some(record_enrichment(container)),
        "research:query-enrichment-history" => Some(query_enrichment_history(container)),
        "research:query-capabilities" => Some(query_capabilities(container)),
        "research:suggest-enrichment" => Some(suggest_enrichment(container)),
        "research:query-frontier" => Some(query_frontier(container)),
        "research:identify-unknown-unknowns" => Some(identify_unknown_unknowns(container)),
        _ => None,
    }
}

fn assess_deficit(container: &Element) -> Result<(), String> {
    let corpus = count_within(container, "[data-corpus-item]")?;
    let dynamics = count_within(container, "[data-research-dynamic]")?;
    let inferences = count_within(container, "[data-research-inference]")?;
    let findings = count_within(container, "[data-research-finding]")?;
    let report = format!(
        "deficit:corpus={corpus};dynamics={dynamics};inferences={inferences};findings={findings}"
    );
    container
        .set_attribute("data-research-deficit", &report)
        .map_err(|_| "Failed to assess research deficit.".to_string())
}

fn identify_frontier(container: &Element) -> Result<(), String> {
    container
        .set_attribute(
            "data-research-frontier",
            "known:scope_defined;unknown:questions_open;unasked:latent_gaps",
        )
        .map_err(|_| "Failed to identify research frontier.".to_string())
}

fn generate_bootstrap_hypotheses(document: &Document, container: &Element) -> Result<(), String> {
    append_nested(
        document,
        container,
        "span",
        "data-bootstrap-hypothesis",
        "H:bootstrap_sketch",
        &[
            ("data-bootstrap-confidence", "low"),
            ("data-bootstrap-status", "provisional"),
        ],
    )
}

fn refine_bootstrap(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-bootstrap-status", "refined")
        .map_err(|_| "Failed to refine bootstrap hypothesis.".to_string())
}

fn reframe_bootstrap(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-bootstrap-status", "reframed")
        .map_err(|_| "Failed to reframe bootstrap hypothesis.".to_string())?;
    let _ = container.set_attribute("data-bootstrap-reframe-reason", "clarity_and_scope");
    Ok(())
}

fn set_bootstrap_status(container: &Element, status: &str) -> Result<(), String> {
    container
        .set_attribute("data-bootstrap-status", status)
        .map_err(|_| format!("Failed to set bootstrap status to {status}."))
}

fn supersede_bootstrap(container: &Element) -> Result<(), String> {
    append_csv_attr(
        container,
        "data-bootstrap-superseded",
        "H:bootstrap_sketch->H:successor",
    )
}

fn promote_bootstrap(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-bootstrap-promoted", "case_hypothesis:H1")
        .map_err(|_| "Failed to promote bootstrap hypothesis.".to_string())
}

fn record_enrichment(container: &Element) -> Result<(), String> {
    append_csv_attr(
        container,
        "data-enrichment-history",
        "enrichment:source_added;unlocked:corpus_analysis",
    )
}

fn query_enrichment_history(container: &Element) -> Result<(), String> {
    let history = container
        .get_attribute("data-enrichment-history")
        .unwrap_or_default();
    let count = if history.is_empty() {
        0
    } else {
        history.split(';').count() as u32
    };
    container
        .set_attribute("data-enrichment-count", &count.to_string())
        .map_err(|_| "Failed to query enrichment history.".to_string())
}

fn query_capabilities(container: &Element) -> Result<(), String> {
    let corpus = count_within(container, "[data-corpus-item]")?;
    let dynamics = count_within(container, "[data-research-dynamic]")?;
    let caps = if corpus > 0 && dynamics > 0 {
        "capabilities:corpus_query,dynamics_overlay,inference_chain"
    } else if corpus > 0 {
        "capabilities:corpus_query,extract_from_corpus"
    } else {
        "capabilities:scope_and_questions_only"
    };
    container
        .set_attribute("data-research-capabilities", caps)
        .map_err(|_| "Failed to query capabilities.".to_string())
}

fn suggest_enrichment(container: &Element) -> Result<(), String> {
    let corpus = count_within(container, "[data-corpus-item]")?;
    let suggestion = if corpus == 0 {
        "suggest:add_corpus_items"
    } else {
        "suggest:refine_bootstrap_hypotheses"
    };
    container
        .set_attribute("data-enrichment-suggestion", suggestion)
        .map_err(|_| "Failed to suggest enrichment.".to_string())
}

fn query_frontier(container: &Element) -> Result<(), String> {
    let frontier = container
        .get_attribute("data-research-frontier")
        .unwrap_or_else(|| "frontier:unset".to_string());
    container
        .set_attribute("data-frontier-query", &frontier)
        .map_err(|_| "Failed to query frontier.".to_string())
}

fn identify_unknown_unknowns(container: &Element) -> Result<(), String> {
    let questions = container
        .get_attribute("data-research-questions")
        .unwrap_or_default();
    let unknowns = if questions.is_empty() {
        "unknown_unknowns:scope_blind_spots_by_analogy"
    } else {
        "unknown_unknowns:latent_questions_from_gaps"
    };
    container
        .set_attribute("data-unknown-unknowns", unknowns)
        .map_err(|_| "Failed to identify unknown unknowns.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_actions_route_safely() {
        assert!(run(
            &web_sys::Document::from(wasm_bindgen::JsValue::NULL),
            &web_sys::Element::from(wasm_bindgen::JsValue::NULL),
            "research:unknown",
        )
        .is_none());
    }
}
