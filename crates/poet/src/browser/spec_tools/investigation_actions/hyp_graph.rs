//! Living hypothesis graph — versioned multi-agent contributions.

use super::shared::{append_csv_attr, append_semicolon_attr, count_selector};
use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "investigation:create-hypothesis-graph" => Some(create_graph(container)),
        "investigation:add-to-graph" => Some(add_to_graph(container)),
        "investigation:contribute-evaluation" => Some(contribute_evaluation(container)),
        "investigation:contribute-confidence-revision" => Some(contribute_confidence_revision(container)),
        "investigation:bridge-dark-link" => Some(bridge_dark_link(container)),
        "investigation:reframe-hypothesis" => Some(reframe_hypothesis(container)),
        "investigation:merge-hypotheses" => Some(merge_hypotheses(container)),
        "investigation:split-hypothesis" => Some(split_hypothesis(container)),
        "investigation:flag-gap" => Some(flag_gap(container)),
        "investigation:close-gap" => Some(close_gap(container)),
        "investigation:merge-contribution" => Some(merge_contribution(container)),
        "investigation:reject-contribution" => Some(reject_contribution(container)),
        "investigation:adjudicate-conflict" => Some(adjudicate_conflict(container)),
        "investigation:create-revision" => Some(create_revision(container)),
        "investigation:diff-revisions" => Some(diff_revisions(container)),
        "investigation:rollback-revision" => Some(rollback_revision(container)),
        "investigation:query-graph" => Some(query_graph(document, container)),
        "investigation:query-gaps" => Some(query_gaps(container)),
        "investigation:query-contributions" => Some(query_contributions(container)),
        "investigation:query-dark-link-bridges" => Some(query_dark_link_bridges(container)),
        "investigation:compute-confidence" => Some(compute_confidence(container)),
        "investigation:rank-graph-hypotheses" => Some(rank_graph_hypotheses(container)),
        "investigation:visualise-graph" => Some(visualise_graph(container)),
        "investigation:subscribe-updates" => Some(subscribe_updates(container)),
        "investigation:unsubscribe-updates" => Some(unsubscribe_updates(container)),
        "investigation:export-graph" => Some(export_graph(container)),
        _ => None,
    }
}

pub(crate) fn next_gap_kind(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("no-hypothesis") => "no-evidence",
        Some("no-evidence") => "conflicting",
        Some("conflicting") => "insufficient",
        Some("insufficient") => "stale",
        Some("stale") => "dark-link-unresolved",
        _ => "no-hypothesis",
    }
}

fn create_graph(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-hypothesis-graph-id", "graph:active")
        .map_err(|_| "Failed to create hypothesis graph.".to_string())?;
    let _ = container.set_attribute("data-hypothesis-graph-revision", "rev:0");
    Ok(())
}

fn add_to_graph(container: &Element) -> Result<(), String> {
    append_csv_attr(container, "data-graph-hypotheses", "H:contribution")
}

fn contribute_evaluation(container: &Element) -> Result<(), String> {
    append_semicolon_attr(
        container,
        "data-graph-evaluations",
        "H:active|evidence:active|verdict:supports|agent:local",
    )
}

fn contribute_confidence_revision(container: &Element) -> Result<(), String> {
    append_semicolon_attr(
        container,
        "data-graph-confidence-revisions",
        "H:active|confidence:0.75|agent:local",
    )
}

fn bridge_dark_link(container: &Element) -> Result<(), String> {
    append_semicolon_attr(
        container,
        "data-graph-dark-bridges",
        "dark_link:research|H:active|verdict:conditional",
    )
}

fn reframe_hypothesis(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-graph-reframe", "H:active|statement:revised|agent:local")
        .map_err(|_| "Failed to reframe hypothesis.".to_string())
}

fn merge_hypotheses(container: &Element) -> Result<(), String> {
    append_semicolon_attr(container, "data-graph-merges", "H1+H2->H_merged")
}

fn split_hypothesis(container: &Element) -> Result<(), String> {
    append_semicolon_attr(container, "data-graph-splits", "H:active->H_a,H_b")
}

fn flag_gap(container: &Element) -> Result<(), String> {
    let kind = next_gap_kind(container.get_attribute("data-graph-gap-kind").as_deref());
    container
        .set_attribute("data-graph-gap-kind", kind)
        .map_err(|_| "Failed to flag graph gap.".to_string())?;
    append_semicolon_attr(container, "data-graph-gaps", &format!("gap:{kind}"))
}

fn close_gap(container: &Element) -> Result<(), String> {
    append_semicolon_attr(container, "data-graph-gap-resolutions", "gap:closed|resolution:documented")
}

fn merge_contribution(container: &Element) -> Result<(), String> {
    append_semicolon_attr(container, "data-graph-contributions-merged", "contrib:pending->applied")
}

fn reject_contribution(container: &Element) -> Result<(), String> {
    append_semicolon_attr(
        container,
        "data-graph-contributions-rejected",
        "contrib:pending|reason:out_of_scope",
    )
}

fn adjudicate_conflict(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-graph-adjudication", "conflict:settled|winner:contrib_a")
        .map_err(|_| "Failed to adjudicate contribution conflict.".to_string())
}

fn create_revision(container: &Element) -> Result<(), String> {
    let rev = container
        .get_attribute("data-hypothesis-graph-revision")
        .and_then(|r| r.strip_prefix("rev:").and_then(|n| n.parse::<u32>().ok()))
        .unwrap_or(0)
        + 1;
    container
        .set_attribute("data-hypothesis-graph-revision", &format!("rev:{rev}"))
        .map_err(|_| "Failed to snapshot hypothesis graph.".to_string())
}

fn diff_revisions(container: &Element) -> Result<(), String> {
    let rev = container
        .get_attribute("data-hypothesis-graph-revision")
        .unwrap_or_else(|| "rev:0".to_string());
    container
        .set_attribute("data-graph-diff", &format!("diff:0->{rev}|changes:hypotheses,evaluations"))
        .map_err(|_| "Failed to diff graph revisions.".to_string())
}

fn rollback_revision(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-hypothesis-graph-revision", "rev:0")
        .map_err(|_| "Failed to roll back hypothesis graph.".to_string())
}

fn query_graph(document: &Document, container: &Element) -> Result<(), String> {
    let graphs = count_selector(document, "[data-hypothesis-graph-id]")?;
    container
        .set_attribute("data-graph-query", &format!("graphs:{graphs}"))
        .map_err(|_| "Failed to query hypothesis graph.".to_string())
}

fn query_gaps(container: &Element) -> Result<(), String> {
    let gaps = container
        .get_attribute("data-graph-gaps")
        .map(|g| g.split(';').filter(|s| !s.is_empty()).count())
        .unwrap_or(0);
    container
        .set_attribute("data-graph-gap-count", &gaps.to_string())
        .map_err(|_| "Failed to query graph gaps.".to_string())
}

fn query_contributions(container: &Element) -> Result<(), String> {
    let merged = container
        .get_attribute("data-graph-contributions-merged")
        .map(|c| c.split(';').filter(|s| !s.is_empty()).count())
        .unwrap_or(0);
    container
        .set_attribute("data-graph-contribution-count", &merged.to_string())
        .map_err(|_| "Failed to query graph contributions.".to_string())
}

fn query_dark_link_bridges(container: &Element) -> Result<(), String> {
    let bridges = container
        .get_attribute("data-graph-dark-bridges")
        .map(|b| b.split(';').filter(|s| !s.is_empty()).count())
        .unwrap_or(0);
    container
        .set_attribute("data-graph-bridge-count", &bridges.to_string())
        .map_err(|_| "Failed to query dark link bridges.".to_string())
}

fn compute_confidence(container: &Element) -> Result<(), String> {
    let evals = container
        .get_attribute("data-graph-evaluations")
        .map(|e| e.split(';').filter(|s| !s.is_empty()).count())
        .unwrap_or(0);
    let score = (evals as f32 * 0.15).min(1.0);
    container
        .set_attribute("data-graph-confidence", &format!("score:{score:.2}"))
        .map_err(|_| "Failed to compute graph confidence.".to_string())
}

fn rank_graph_hypotheses(container: &Element) -> Result<(), String> {
    let hyps = container
        .get_attribute("data-graph-hypotheses")
        .unwrap_or_else(|| "H:active".to_string());
    container
        .set_attribute("data-graph-rank", &format!("ranked:{hyps}"))
        .map_err(|_| "Failed to rank graph hypotheses.".to_string())
}

fn visualise_graph(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-graph-layout", "layout:confidence_weighted")
        .map_err(|_| "Failed to visualise hypothesis graph.".to_string())
}

fn subscribe_updates(container: &Element) -> Result<(), String> {
    append_csv_attr(container, "data-graph-subscribers", "agent:local")
}

fn unsubscribe_updates(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-graph-subscribers", "")
        .map_err(|_| "Failed to unsubscribe from graph updates.".to_string())
}

fn export_graph(container: &Element) -> Result<(), String> {
    let rev = container
        .get_attribute("data-hypothesis-graph-revision")
        .unwrap_or_else(|| "rev:0".to_string());
    let hyps = container
        .get_attribute("data-graph-hypotheses")
        .unwrap_or_default();
    let export = format!("{{\"revision\":\"{rev}\",\"hypotheses\":\"{hyps}\"}}");
    container
        .set_attribute("data-graph-export", &export)
        .map_err(|_| "Failed to export hypothesis graph.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gap_kinds_cycle() {
        assert_eq!(next_gap_kind(None), "no-hypothesis");
        assert_eq!(next_gap_kind(Some("stale")), "dark-link-unresolved");
        assert_eq!(next_gap_kind(Some("dark-link-unresolved")), "no-hypothesis");
    }
}
