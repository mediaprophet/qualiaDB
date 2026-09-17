//! Case lifecycle, entity registration, and reporting for Poet investigations.

use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "investigation:new-investigation" => Some(new_investigation(container)),
        "investigation:set-mode" => Some(cycle_mode(container)),
        "investigation:set-status" => Some(cycle_status(container)),
        "investigation:set-jurisdiction" => Some(set_jurisdiction(container)),
        "investigation:add-subject" => Some(add_subject(container)),
        "investigation:add-topic" => Some(add_topic(container)),
        "investigation:add-event" => Some(add_event(container)),
        "investigation:add-case" => Some(add_case(container)),
        "investigation:add-constituency" => Some(add_constituency(container)),
        "investigation:query-investigation" => Some(query_investigation(document, container)),
        "investigation:export-findings" => Some(export_findings(container)),
        _ => None,
    }
}

pub(crate) fn next_case_mode(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("retrospective") => "prospective",
        Some("prospective") => "hybrid",
        _ => "retrospective",
    }
}

pub(crate) fn next_case_status(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("open") => "active",
        Some("active") => "paused",
        Some("paused") => "closed",
        Some("closed") => "cold",
        _ => "open",
    }
}

fn new_investigation(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-case-id", "case:active_investigation")
        .map_err(|_| "Failed to open new investigation case.".to_string())?;
    let _ = container.set_attribute("data-case-status", "open");
    let _ = container.set_attribute("data-case-mode", "retrospective");
    Ok(())
}

fn cycle_mode(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-case-mode");
    let next = next_case_mode(current.as_deref());
    container
        .set_attribute("data-case-mode", next)
        .map_err(|_| "Failed to update case mode.".to_string())
}

fn cycle_status(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-case-status");
    let next = next_case_status(current.as_deref());
    container
        .set_attribute("data-case-status", next)
        .map_err(|_| "Failed to update case status.".to_string())
}

fn set_jurisdiction(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-case-jurisdiction", "legal:qualitative_commons")
        .map_err(|_| "Failed to set jurisdiction.".to_string())
}

fn add_subject(container: &Element) -> Result<(), String> {
    append_csv_attr(container, "data-case-subjects", "entity:subject")
}

fn add_topic(container: &Element) -> Result<(), String> {
    append_csv_attr(container, "data-case-topics", "topic:investigation")
}

fn add_event(container: &Element) -> Result<(), String> {
    append_csv_attr(container, "data-case-events", "event:timeline_node")
}

fn add_case(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-case-file-attached", "true")
        .map_err(|_| "Failed to attach case file.".to_string())
}

fn add_constituency(container: &Element) -> Result<(), String> {
    append_csv_attr(container, "data-case-constituents", "party:stakeholder")
}

fn query_investigation(document: &Document, container: &Element) -> Result<(), String> {
    let cases = document
        .query_selector_all("[data-case-id]")
        .map_err(|_| "Failed to query investigation cases.".to_string())?;
    container
        .set_attribute("data-investigation-matches", &cases.length().to_string())
        .map_err(|_| "Failed to record case match count.".to_string())
}

fn export_findings(container: &Element) -> Result<(), String> {
    let status = container
        .get_attribute("data-case-status")
        .unwrap_or_else(|| "open".to_string());
    let mode = container
        .get_attribute("data-case-mode")
        .unwrap_or_else(|| "retrospective".to_string());
    let export = format!("{{\"case\":\"active\",\"status\":\"{status}\",\"mode\":\"{mode}\"}}");
    container
        .set_attribute("data-case-export", &export)
        .map_err(|_| "Failed to export case findings.".to_string())
}

fn append_csv_attr(container: &Element, attr: &str, item: &str) -> Result<(), String> {
    let current = container.get_attribute(attr).unwrap_or_default();
    let updated = if current.is_empty() {
        item.to_string()
    } else {
        format!("{current},{item}")
    };
    container
        .set_attribute(attr, &updated)
        .map_err(|_| format!("Failed to update {attr}."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_mode_and_status_cycle() {
        assert_eq!(next_case_mode(None), "retrospective");
        assert_eq!(next_case_mode(Some("retrospective")), "prospective");
        assert_eq!(next_case_mode(Some("prospective")), "hybrid");
        assert_eq!(next_case_mode(Some("hybrid")), "retrospective");

        assert_eq!(next_case_status(None), "open");
        assert_eq!(next_case_status(Some("open")), "active");
        assert_eq!(next_case_status(Some("active")), "paused");
        assert_eq!(next_case_status(Some("paused")), "closed");
        assert_eq!(next_case_status(Some("closed")), "cold");
        assert_eq!(next_case_status(Some("cold")), "open");
    }
}
