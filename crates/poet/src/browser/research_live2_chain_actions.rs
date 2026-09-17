//! Dual-path Tool Chest remainder for Host-bound `Research.*` ids (wave 28).
//!
//! Investigation, hypothesis-graph, and epistemic-assessment binds on
//! `research:live`. Local sketches record Host status-record args.

use serde_json::json;
use web_sys::{Document, Element};

const DEFAULT_ID: &str = "enquiry-1";

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn selected_source(document: &Document) -> Option<String> {
    let container = selected_container(document)?;
    let text = container
        .query_selector(".vibe-editor, .vibe-editor-textarea, .doc-editor, .sheet-grid")
        .ok()
        .flatten()
        .and_then(|editor| editor.text_content())
        .or_else(|| container.text_content())?;
    let bounded: String = text.chars().take(8_192).collect();
    (!bounded.trim().is_empty()).then_some(bounded)
}

fn string_attr(el: Option<&Element>, name: &str) -> Option<String> {
    el.and_then(|e| e.get_attribute(name))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn first_line(source: &str) -> String {
    source
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_string()
}

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: serde_json::Value,
) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(cap_id, &local_message);
        super::interactions::show_tool_status(document, &label, &report.message, report.status_kind);
        return;
    }
    super::interactions::show_tool_status(document, &label, &format!("Running {cap_id}…"), "running");
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match super::native_daemon::daemon_invoke(cap_id, args).await {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok(cap_id, &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    cap_id,
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("capability invoke failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied(cap_id, &error);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
        }
    });
}

fn enquiry_id(document: &Document) -> String {
    string_attr(selected_container(document).as_ref(), "data-research-id")
        .unwrap_or_else(|| DEFAULT_ID.to_string())
}

fn surface_text(document: &Document) -> String {
    selected_source(document).unwrap_or_default()
}

fn run_merge(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    sketch: &str,
    mut extra: serde_json::Value,
) {
    let id = enquiry_id(document);
    if let Some(obj) = extra.as_object_mut() {
        obj.entry("id".to_string())
            .or_insert_with(|| json!(id.clone()));
    }
    invoke_dual(document, label, cap_id, format!("{sketch} id={id}"), extra);
}

fn run_node(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    sketch: &str,
    mut extra: serde_json::Value,
) {
    let id = enquiry_id(document);
    if let Some(obj) = extra.as_object_mut() {
        obj.entry("node_id".to_string())
            .or_insert_with(|| json!(id.clone()));
    }
    invoke_dual(document, label, cap_id, format!("{sketch} node={id}"), extra);
}

pub(super) fn run_new_investigation(document: &Document, label: &str) {
    run_merge(
        document,
        label,
        "Research.new_investigation",
        "new_investigation sketch",
        json!({}),
    );
}

pub(super) fn run_collect_evidence(document: &Document, label: &str) {
    let description = first_line(&surface_text(document));
    run_merge(
        document,
        label,
        "Research.collect_evidence",
        "collect_evidence sketch",
        json!({
            "description": if description.is_empty() { "evidence".into() } else { description },
            "source": "surface",
        }),
    );
}

pub(super) fn run_set_reliability(document: &Document, label: &str) {
    let reliability = string_attr(selected_container(document).as_ref(), "data-reliability")
        .unwrap_or_else(|| "possible".into());
    run_merge(
        document,
        label,
        "Research.set_reliability",
        "set_reliability sketch",
        json!({ "reliability": reliability }),
    );
}

pub(super) fn run_propose_hypothesis(document: &Document, label: &str) {
    let statement = first_line(&surface_text(document));
    run_merge(
        document,
        label,
        "Research.propose_hypothesis",
        "propose_hypothesis sketch",
        json!({ "statement": if statement.is_empty() { "hypothesis".into() } else { statement } }),
    );
}

pub(super) fn run_evaluate_evidence(document: &Document, label: &str) {
    run_merge(
        document,
        label,
        "Research.evaluate_evidence",
        "evaluate_evidence sketch",
        json!({ "hypothesis_id": "h1" }),
    );
}

pub(super) fn run_create_timeline(document: &Document, label: &str) {
    let event = first_line(&surface_text(document));
    run_merge(
        document,
        label,
        "Research.create_timeline",
        "create_timeline sketch",
        json!({
            "timestamp": "t0",
            "event": if event.is_empty() { "event".into() } else { event },
        }),
    );
}

pub(super) fn run_add_link(document: &Document, label: &str) {
    run_merge(
        document,
        label,
        "Research.add_link",
        "add_link sketch",
        json!({ "source": "a", "target": "b", "link_type": "related" }),
    );
}

pub(super) fn run_find_path(document: &Document, label: &str) {
    run_merge(
        document,
        label,
        "Research.find_path",
        "find_path sketch",
        json!({ "start": "a", "end": "b" }),
    );
}

pub(super) fn run_create_hypothesis_graph(document: &Document, label: &str) {
    run_merge(
        document,
        label,
        "Research.create_hypothesis_graph",
        "create_hypothesis_graph sketch",
        json!({}),
    );
}

pub(super) fn run_contribute_evaluation(document: &Document, label: &str) {
    run_node(
        document,
        label,
        "Research.contribute_evaluation",
        "contribute_evaluation sketch",
        json!({ "agent_id": "poet", "score": 0.5 }),
    );
}

pub(super) fn run_bridge_dark_link(document: &Document, label: &str) {
    run_node(
        document,
        label,
        "Research.bridge_dark_link",
        "bridge_dark_link sketch",
        json!({ "dark_link_id": "dl1" }),
    );
}

pub(super) fn run_reframe_hypothesis(document: &Document, label: &str) {
    let statement = first_line(&surface_text(document));
    run_node(
        document,
        label,
        "Research.reframe_hypothesis",
        "reframe_hypothesis sketch",
        json!({ "new_statement": if statement.is_empty() { "reframed".into() } else { statement } }),
    );
}

pub(super) fn run_merge_hypotheses(document: &Document, label: &str) {
    let merged = first_line(&surface_text(document));
    invoke_dual(
        document,
        label,
        "Research.merge_hypotheses",
        "merge_hypotheses sketch".into(),
        json!({
            "node1_id": "n1",
            "node2_id": "n2",
            "merged_statement": if merged.is_empty() { "merged".into() } else { merged },
        }),
    );
}

pub(super) fn run_flag_gap(document: &Document, label: &str) {
    let gap = first_line(&surface_text(document));
    run_node(
        document,
        label,
        "Research.flag_gap",
        "flag_gap sketch",
        json!({ "gap": if gap.is_empty() { "gap".into() } else { gap } }),
    );
}

pub(super) fn run_close_gap(document: &Document, label: &str) {
    let gap = first_line(&surface_text(document));
    run_node(
        document,
        label,
        "Research.close_gap",
        "close_gap sketch",
        json!({ "gap": if gap.is_empty() { "gap".into() } else { gap } }),
    );
}

pub(super) fn run_create_revision(document: &Document, label: &str) {
    let statement = first_line(&surface_text(document));
    run_node(
        document,
        label,
        "Research.create_revision",
        "create_revision sketch",
        json!({ "new_statement": if statement.is_empty() { "revision".into() } else { statement } }),
    );
}

pub(super) fn run_diff_revisions(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Research.diff_revisions",
        "diff_revisions sketch".into(),
        json!({ "rev1_id": "rev_0", "rev2_id": "rev_1" }),
    );
}

pub(super) fn run_subscribe_updates(document: &Document, label: &str) {
    run_node(
        document,
        label,
        "Research.subscribe_updates",
        "subscribe_updates sketch",
        json!({ "agent_id": "poet" }),
    );
}

pub(super) fn run_create_assessment(document: &Document, label: &str) {
    let content_ref = first_line(&surface_text(document));
    run_merge(
        document,
        label,
        "Research.create_assessment",
        "create_assessment sketch",
        json!({ "content_ref": if content_ref.is_empty() { "surface".into() } else { content_ref } }),
    );
}

pub(super) fn run_set_epistemic_mode(document: &Document, label: &str) {
    let mode = string_attr(selected_container(document).as_ref(), "data-epistemic-mode")
        .unwrap_or_else(|| "empirical".into());
    run_merge(
        document,
        label,
        "Research.set_epistemic_mode",
        "set_epistemic_mode sketch",
        json!({ "mode": mode }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave28_research_defaults() {
        assert_eq!(DEFAULT_ID, "enquiry-1");
        assert_eq!(first_line("  note\n"), "note");
    }
}
