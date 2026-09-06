//! Findings, syntheses, and research outputs for Poet research.

use super::util::{append_csv_attr, append_nested, count_document, count_within, next_confidence};
use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "research:create-finding" => Some(create_finding(document, container)),
        "research:set-finding-confidence" => Some(set_finding_confidence(container)),
        "research:mark-finding-contested" => Some(toggle_finding_contested(container)),
        "research:link-finding-to-question" => Some(link_finding_to_question(container)),
        "research:create-synthesis" => Some(create_synthesis(document, container)),
        "research:add-finding-to-synthesis" => Some(add_finding_to_synthesis(container)),
        "research:query-findings" => Some(query_findings(document, container)),
        "research:query-syntheses" => Some(query_syntheses(document, container)),
        "research:export-synthesis" => Some(export_synthesis(container)),
        _ => None,
    }
}

fn create_finding(document: &Document, container: &Element) -> Result<(), String> {
    append_nested(
        document,
        container,
        "span",
        "data-research-finding",
        "finding:evidence_backed",
        &[("data-finding-confidence", "moderate")],
    )
}

fn set_finding_confidence(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-finding-confidence");
    let next = next_confidence(current.as_deref());
    container
        .set_attribute("data-finding-confidence", next)
        .map_err(|_| "Failed to set finding confidence.".to_string())
}

fn toggle_finding_contested(container: &Element) -> Result<(), String> {
    let contested = container
        .get_attribute("data-finding-contested")
        .is_some_and(|v| v == "true");
    if contested {
        let _ = container.remove_attribute("data-finding-contested");
    } else {
        container
            .set_attribute("data-finding-contested", "true")
            .map_err(|_| "Failed to mark finding contested.".to_string())?;
    }
    Ok(())
}

fn link_finding_to_question(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-finding-question-link", "finding -> answers -> Q:active_enquiry")
        .map_err(|_| "Failed to link finding to question.".to_string())
}

fn create_synthesis(document: &Document, container: &Element) -> Result<(), String> {
    append_nested(
        document,
        container,
        "span",
        "data-research-synthesis",
        "synthesis:narrative_review",
        &[("data-synthesis-type", "systematic_review")],
    )
}

fn add_finding_to_synthesis(container: &Element) -> Result<(), String> {
    append_csv_attr(
        container,
        "data-synthesis-findings",
        "finding:evidence_backed",
    )
}

fn query_findings(document: &Document, container: &Element) -> Result<(), String> {
    let local = count_within(container, "[data-research-finding]")?;
    let global = count_document(document, "[data-research-finding]")?;
    container
        .set_attribute(
            "data-finding-query",
            &format!("local={local};canvas={global}"),
        )
        .map_err(|_| "Failed to query findings.".to_string())
}

fn query_syntheses(document: &Document, container: &Element) -> Result<(), String> {
    let local = count_within(container, "[data-research-synthesis]")?;
    let global = count_document(document, "[data-research-synthesis]")?;
    container
        .set_attribute(
            "data-synthesis-query",
            &format!("local={local};canvas={global}"),
        )
        .map_err(|_| "Failed to query syntheses.".to_string())
}

fn export_synthesis(container: &Element) -> Result<(), String> {
    let synth_type = container
        .get_attribute("data-synthesis-type")
        .unwrap_or_else(|| "narrative".to_string());
    let findings = container
        .get_attribute("data-synthesis-findings")
        .unwrap_or_default();
    let export = format!(
        "{{\"type\":\"{synth_type}\",\"findings\":\"{findings}\",\"format\":\"report\"}}"
    );
    container
        .set_attribute("data-synthesis-export", &export)
        .map_err(|_| "Failed to export synthesis.".to_string())
}
