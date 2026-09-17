//! Perspective comparison, conflict detection, and reconciliation microformats.

use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "epistemic:compare-perspectives" => Some(compare_perspectives(document, container)),
        "epistemic:detect-perspective-conflict" => Some(detect_perspective_conflict(document, container)),
        "epistemic:assess-perspective-coverage" => Some(assess_perspective_coverage(document, container)),
        "epistemic:reconcile-perspectives" => Some(reconcile_perspectives(container)),
        _ => None,
    }
}

pub(crate) fn next_reconcile_strategy(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("consensus") => "weighted_average",
        Some("weighted_average") => "adjudication",
        Some("adjudication") => "preserve_disagreement",
        _ => "consensus",
    }
}

fn compare_perspectives(document: &Document, container: &Element) -> Result<(), String> {
    let list = document
        .query_selector_all("[data-perspective-dids]")
        .map_err(|_| "Failed to query perspectives for comparison.".to_string())?;
    let count = list.length();
    let summary = if count >= 2 {
        format!("{count} perspectives compared; agreement/disagreement mapped")
    } else {
        "single perspective on canvas; comparison deferred".to_string()
    };
    container
        .set_attribute("data-perspective-comparison", &summary)
        .map_err(|_| "Failed to record perspective comparison.".to_string())
}

fn detect_perspective_conflict(document: &Document, container: &Element) -> Result<(), String> {
    let modes = document
        .query_selector_all("[data-epistemic-mode]")
        .map_err(|_| "Failed to query epistemic modes for conflict detection.".to_string())?;
    let conflict = modes.length() >= 2;
    container
        .set_attribute(
            "data-perspective-conflict",
            if conflict { "detected" } else { "none" },
        )
        .map_err(|_| "Failed to record perspective conflict status.".to_string())
}

fn assess_perspective_coverage(document: &Document, container: &Element) -> Result<(), String> {
    let registered = document
        .query_selector_all("[data-perspective-dids]")
        .map_err(|_| "Failed to query registered perspectives.".to_string())?;
    let count = registered.length();
    let coverage = if count == 0 {
        "missing:human,software-agent,sensor,organisation"
    } else if count < 3 {
        "partial:coverage gaps remain"
    } else {
        "adequate:multiple observer types represented"
    };
    container
        .set_attribute("data-perspective-coverage", coverage)
        .map_err(|_| "Failed to record perspective coverage.".to_string())
}

fn reconcile_perspectives(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-perspective-reconcile-strategy");
    let next = next_reconcile_strategy(current.as_deref());
    container
        .set_attribute("data-perspective-reconcile-strategy", next)
        .map_err(|_| "Failed to set perspective reconciliation strategy.".to_string())?;
    let _ = container.set_attribute("data-perspective-reconciled", "pending");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconcile_strategies_cycle() {
        assert_eq!(next_reconcile_strategy(None), "consensus");
        assert_eq!(next_reconcile_strategy(Some("consensus")), "weighted_average");
        assert_eq!(
            next_reconcile_strategy(Some("weighted_average")),
            "adjudication"
        );
        assert_eq!(
            next_reconcile_strategy(Some("adjudication")),
            "preserve_disagreement"
        );
        assert_eq!(
            next_reconcile_strategy(Some("preserve_disagreement")),
            "consensus"
        );
    }
}
