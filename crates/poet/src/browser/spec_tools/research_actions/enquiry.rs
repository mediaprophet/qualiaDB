//! Research enquiry, purpose, scope, and question management for Poet containers.

use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "research:new-research" => Some(new_research(container)),
        "research:set-purpose" => Some(cycle_purpose(container)),
        "research:define-scope" => Some(define_scope(container)),
        "research:add-constraint" => Some(add_constraint(container)),
        "research:add-question" => Some(add_question(container)),
        "research:link-questions" => Some(link_questions(container)),
        "research:set-question-status" => Some(cycle_question_status(container)),
        "research:link-investigation" => Some(link_investigation(container)),
        "research:set-research-status" => Some(cycle_research_status(container)),
        "research:query-research" => Some(query_research(document, container)),
        _ => None,
    }
}

pub(crate) fn next_research_purpose(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("exploratory") => "explanatory",
        Some("explanatory") => "evaluative",
        Some("evaluative") => "generative",
        Some("generative") => "confirmatory",
        Some("confirmatory") => "integrative",
        Some("integrative") => "critical",
        _ => "exploratory",
    }
}

pub(crate) fn next_research_status(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("proposing") => "active",
        Some("active") => "paused",
        Some("paused") => "concluded",
        Some("concluded") => "superseded",
        _ => "proposing",
    }
}

pub(crate) fn next_question_status(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("open") => "in_progress",
        Some("in_progress") => "answered",
        Some("answered") => "reframed",
        Some("reframed") => "deferred",
        _ => "open",
    }
}

fn new_research(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-research-project", "project:active")
        .map_err(|_| "Failed to open new research project.".to_string())?;
    let _ = container.set_attribute("data-research-purpose", "exploratory");
    let _ = container.set_attribute("data-research-status", "active");
    Ok(())
}

fn cycle_purpose(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-research-purpose");
    let next = next_research_purpose(current.as_deref());
    container
        .set_attribute("data-research-purpose", next)
        .map_err(|_| "Failed to update research purpose.".to_string())
}

fn define_scope(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-research-scope", "bounds:spatial_temporal_methodological")
        .map_err(|_| "Failed to define research scope.".to_string())
}

fn add_constraint(container: &Element) -> Result<(), String> {
    append_csv_attr(container, "data-research-constraints", "constraint:ethical_and_resource")
}

fn add_question(container: &Element) -> Result<(), String> {
    append_csv_attr(container, "data-research-questions", "Q:active_enquiry")
}

fn link_questions(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-question-dependency", "Q1 -> informs -> Q2")
        .map_err(|_| "Failed to link research questions.".to_string())
}

fn cycle_question_status(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-question-status");
    let next = next_question_status(current.as_deref());
    container
        .set_attribute("data-question-status", next)
        .map_err(|_| "Failed to update question status.".to_string())
}

fn link_investigation(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-linked-case", "case:active_investigation")
        .map_err(|_| "Failed to link case to research project.".to_string())
}

fn cycle_research_status(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-research-status");
    let next = next_research_status(current.as_deref());
    container
        .set_attribute("data-research-status", next)
        .map_err(|_| "Failed to update research project status.".to_string())
}

fn query_research(document: &Document, container: &Element) -> Result<(), String> {
    let projects = document
        .query_selector_all("[data-research-project]")
        .map_err(|_| "Failed to query research projects.".to_string())?;
    container
        .set_attribute("data-research-count", &projects.length().to_string())
        .map_err(|_| "Failed to record research project count.".to_string())
}

fn append_csv_attr(container: &Element, attr: &str, item: &str) -> Result<(), String> {
    let current = container.get_attribute(attr).unwrap_or_default();
    let updated = if current.is_empty() {
        item.to_string()
    } else {
        format!("{current};{item}")
    };
    container
        .set_attribute(attr, &updated)
        .map_err(|_| format!("Failed to update {attr}."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_purpose_and_status_cycle() {
        assert_eq!(next_research_purpose(None), "exploratory");
        assert_eq!(next_research_purpose(Some("exploratory")), "explanatory");
        assert_eq!(next_research_purpose(Some("explanatory")), "evaluative");

        assert_eq!(next_research_status(None), "proposing");
        assert_eq!(next_research_status(Some("proposing")), "active");
        assert_eq!(next_research_status(Some("active")), "paused");

        assert_eq!(next_question_status(None), "open");
        assert_eq!(next_question_status(Some("open")), "in_progress");
        assert_eq!(next_question_status(Some("in_progress")), "answered");
    }
}
