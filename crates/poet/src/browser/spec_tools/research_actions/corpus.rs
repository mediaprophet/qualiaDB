//! Corpus management, empirical findings, and literature synthesis for Poet research.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "research:add-corpus-item" => Some(add_corpus_item(container)),
        "research:extract-finding" => Some(extract_finding(container)),
        "research:tag-methodology" => Some(tag_methodology(container)),
        "research:export-research" => Some(export_research(container)),
        "research:synthesize-literature" => Some(synthesize_literature(container)),
        "research:add-finding" => Some(add_finding(container)),
        "research:assess-evidence-quality" => Some(assess_evidence_quality(container)),
        _ => None,
    }
}

fn add_corpus_item(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-corpus-items")
        .unwrap_or_default();
    let entry = "source:primary_literature";
    let updated = if current.is_empty() {
        entry.to_string()
    } else {
        format!("{current};{entry}")
    };
    container
        .set_attribute("data-corpus-items", &updated)
        .map_err(|_| "Failed to append corpus item.".to_string())
}

fn extract_finding(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-research-findings")
        .unwrap_or_default();
    let entry = "finding:empirical_result";
    let updated = if current.is_empty() {
        entry.to_string()
    } else {
        format!("{current};{entry}")
    };
    container
        .set_attribute("data-research-findings", &updated)
        .map_err(|_| "Failed to extract research finding.".to_string())
}

fn tag_methodology(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-research-methodology", "method:triangulated_empirical")
        .map_err(|_| "Failed to tag research methodology.".to_string())
}

fn export_research(container: &Element) -> Result<(), String> {
    let purpose = container
        .get_attribute("data-research-purpose")
        .unwrap_or_else(|| "exploratory".to_string());
    let status = container
        .get_attribute("data-research-status")
        .unwrap_or_else(|| "active".to_string());
    let export = format!("{{\"project\":\"active\",\"purpose\":\"{purpose}\",\"status\":\"{status}\"}}");
    container
        .set_attribute("data-research-export", &export)
        .map_err(|_| "Failed to export research project.".to_string())
}

fn synthesize_literature(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-literature-synthesis", "synthesized:consensus_and_gaps_identified")
        .map_err(|_| "Failed to record literature synthesis.".to_string())
}

fn add_finding(container: &Element) -> Result<(), String> {
    extract_finding(container)
}

fn assess_evidence_quality(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-evidence-quality", "GRADE:high_certainty")
        .map_err(|_| "Failed to assess evidence quality.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_actions_route_safely() {
        assert!(run(&web_sys::Element::from(wasm_bindgen::JsValue::NULL), "research:unknown").is_none());
    }
}
