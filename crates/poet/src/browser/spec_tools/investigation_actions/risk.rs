//! Risk assessment and monitoring for investigation scenarios.

use super::shared::count_selector;
use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "investigation:assess-risk" => Some(assess_risk(container)),
        "investigation:build-risk-matrix" => Some(build_risk_matrix(document, container)),
        "investigation:identify-tail-risks" => Some(identify_tail_risks(container)),
        "investigation:set-risk-threshold" => Some(set_risk_threshold(container)),
        "investigation:monitor-risk" => Some(monitor_risk(container)),
        "investigation:risk-report" => Some(risk_report(container)),
        _ => None,
    }
}

pub(crate) fn next_risk_threshold(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("low") => "medium",
        Some("medium") => "high",
        Some("high") => "critical",
        _ => "low",
    }
}

pub(crate) fn probability_score(label: Option<&str>) -> u32 {
    match label.map(str::trim) {
        Some("negligible") => 1,
        Some("low") => 2,
        Some("moderate") => 3,
        Some("high") => 4,
        Some("near-certain") => 5,
        _ => 2,
    }
}

fn assess_risk(container: &Element) -> Result<(), String> {
    let prob = probability_score(container.get_attribute("data-scenario-probability").as_deref());
    let impact = container
        .get_attribute("data-scenario-impact")
        .and_then(|v| v.parse().ok())
        .unwrap_or(3u32);
    let score = prob * impact;
    container
        .set_attribute("data-risk-score", &score.to_string())
        .map_err(|_| "Failed to assess risk.".to_string())
}

fn build_risk_matrix(document: &Document, container: &Element) -> Result<(), String> {
    let scenarios = count_selector(document, "[data-scenario-id]")?;
    container
        .set_attribute("data-risk-matrix", &format!("matrix:scenarios={scenarios}|axes=probability_x_impact"))
        .map_err(|_| "Failed to build risk matrix.".to_string())
}

fn identify_tail_risks(container: &Element) -> Result<(), String> {
    let prob = container.get_attribute("data-scenario-probability");
    let impact = container
        .get_attribute("data-scenario-impact")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);
    let tail = match prob.as_deref() {
        Some("negligible") | Some("low") if impact >= 4 => "tail:low_probability_high_impact",
        _ => "tail:none_identified",
    };
    container
        .set_attribute("data-risk-tail", tail)
        .map_err(|_| "Failed to identify tail risks.".to_string())
}

fn set_risk_threshold(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-risk-threshold");
    let next = next_risk_threshold(current.as_deref());
    container
        .set_attribute("data-risk-threshold", next)
        .map_err(|_| "Failed to set risk threshold.".to_string())
}

fn monitor_risk(container: &Element) -> Result<(), String> {
    let score = container
        .get_attribute("data-risk-score")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    let threshold = container
        .get_attribute("data-risk-threshold")
        .unwrap_or_else(|| "medium".to_string());
    let alert = match (score, threshold.as_str()) {
        (s, "critical") if s >= 12 => "alert:breached",
        (s, "high") if s >= 9 => "alert:breached",
        (s, "medium") if s >= 6 => "alert:breached",
        (s, "low") if s >= 3 => "alert:breached",
        _ => "alert:within_threshold",
    };
    container
        .set_attribute("data-risk-monitor", alert)
        .map_err(|_| "Failed to monitor risk.".to_string())
}

fn risk_report(container: &Element) -> Result<(), String> {
    let score = container
        .get_attribute("data-risk-score")
        .unwrap_or_else(|| "0".to_string());
    let threshold = container
        .get_attribute("data-risk-threshold")
        .unwrap_or_else(|| "medium".to_string());
    let monitor = container
        .get_attribute("data-risk-monitor")
        .unwrap_or_else(|| "unknown".to_string());
    let report = format!("{{\"risk_score\":{score},\"threshold\":\"{threshold}\",\"monitor\":\"{monitor}\"}}");
    container
        .set_attribute("data-risk-report", &report)
        .map_err(|_| "Failed to generate risk report.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_threshold_cycles() {
        assert_eq!(next_risk_threshold(None), "low");
        assert_eq!(next_risk_threshold(Some("critical")), "low");
    }

    #[test]
    fn probability_scores_map_bands() {
        assert_eq!(probability_score(Some("near-certain")), 5);
        assert_eq!(probability_score(None), 2);
    }
}
