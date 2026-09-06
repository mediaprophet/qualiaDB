//! Scenario modelling for forecast investigations.

use super::shared::{append_csv_attr, append_semicolon_attr, count_selector};
use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "investigation:create-scenario" => Some(create_scenario(container)),
        "investigation:set-probability" => Some(cycle_probability(container)),
        "investigation:add-driving-factor" => Some(add_driving_factor(container)),
        "investigation:add-outcome" => Some(add_outcome(container)),
        "investigation:set-scenario-base" => Some(set_scenario_base(container)),
        "investigation:branch-scenario" => Some(branch_scenario(container)),
        "investigation:query-scenarios" => Some(query_scenarios(document, container)),
        "investigation:compare-scenarios" => Some(compare_scenarios(document, container)),
        "investigation:merge-scenarios" => Some(merge_scenarios(container)),
        _ => None,
    }
}

pub(crate) fn next_scenario_kind(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("best-case") => "worst-case",
        Some("worst-case") => "likely",
        Some("likely") => "black-swan",
        Some("black-swan") => "branching",
        _ => "best-case",
    }
}

pub(crate) fn next_probability_band(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("negligible") => "low",
        Some("low") => "moderate",
        Some("moderate") => "high",
        Some("high") => "near-certain",
        _ => "negligible",
    }
}

fn create_scenario(container: &Element) -> Result<(), String> {
    let kind = next_scenario_kind(None);
    container
        .set_attribute("data-scenario-id", "scenario:active")
        .map_err(|_| "Failed to create scenario.".to_string())?;
    let _ = container.set_attribute("data-scenario-kind", kind);
    let _ = container.set_attribute("data-scenario-probability", "negligible");
    Ok(())
}

fn cycle_probability(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-scenario-probability");
    let next = next_probability_band(current.as_deref());
    container
        .set_attribute("data-scenario-probability", next)
        .map_err(|_| "Failed to set scenario probability.".to_string())
}

fn add_driving_factor(container: &Element) -> Result<(), String> {
    append_csv_attr(container, "data-scenario-factors", "factor:event_or_policy")
}

fn add_outcome(container: &Element) -> Result<(), String> {
    append_semicolon_attr(container, "data-scenario-outcomes", "outcome:projected|unit:qualitative")
}

fn set_scenario_base(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-scenario-base", "basis:evidence_and_pattern")
        .map_err(|_| "Failed to link scenario basis.".to_string())
}

fn branch_scenario(container: &Element) -> Result<(), String> {
    append_semicolon_attr(container, "data-scenario-branches", "branch:decision_point->scenario_b")
}

fn query_scenarios(document: &Document, container: &Element) -> Result<(), String> {
    let count = count_selector(document, "[data-scenario-id]")?;
    container
        .set_attribute("data-scenario-count", &count.to_string())
        .map_err(|_| "Failed to query scenarios.".to_string())
}

fn compare_scenarios(document: &Document, container: &Element) -> Result<(), String> {
    let count = count_selector(document, "[data-scenario-id]")?;
    let verdict = if count >= 2 {
        "comparison:side_by_side_ready"
    } else {
        "comparison:need_two_scenarios"
    };
    container
        .set_attribute("data-scenario-compare", verdict)
        .map_err(|_| "Failed to compare scenarios.".to_string())
}

fn merge_scenarios(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-scenario-merged", "composite:multi_scenario_blend")
        .map_err(|_| "Failed to merge scenarios.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_kind_and_probability_cycle() {
        assert_eq!(next_scenario_kind(None), "best-case");
        assert_eq!(next_scenario_kind(Some("branching")), "best-case");
        assert_eq!(next_probability_band(Some("high")), "near-certain");
    }
}
