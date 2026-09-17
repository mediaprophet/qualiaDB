//! Dual-path Tool Chest remainder for Host-bound `Research.*` ids (wave 29).
//!
//! Reality, sentiment, perspective, and intentionality binds on `research:live`.

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

fn content(document: &Document) -> String {
    let t = first_line(&surface_text(document));
    if t.is_empty() {
        "content".into()
    } else {
        t
    }
}

fn run_id(
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

fn run_content(document: &Document, label: &str, cap_id: &'static str, field: &str) {
    let body = content(document);
    invoke_dual(
        document,
        label,
        cap_id,
        format!("{cap_id} sketch"),
        json!({ field: body }),
    );
}

pub(super) fn run_set_reality_category(document: &Document, label: &str) {
    let category = string_attr(selected_container(document).as_ref(), "data-category")
        .unwrap_or_else(|| "uncertain".into());
    run_id(
        document,
        label,
        "Research.set_reality_category",
        "set_reality_category sketch",
        json!({ "category": category }),
    );
}

pub(super) fn run_classify_reality(document: &Document, label: &str) {
    run_content(document, label, "Research.classify_reality", "content");
}

pub(super) fn run_detect_blended(document: &Document, label: &str) {
    run_content(document, label, "Research.detect_blended", "content");
}

pub(super) fn run_detect_deceptive_fiction(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Research.detect_deceptive_fiction",
        "detect_deceptive_fiction sketch".into(),
        json!({ "content": content(document), "claimed_category": "factual" }),
    );
}

pub(super) fn run_trace_fiction(document: &Document, label: &str) {
    let lines: Vec<String> = surface_text(document)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .take(16)
        .collect();
    let fiction = lines.first().cloned().unwrap_or_else(|| "fiction".into());
    let corpus = if lines.len() > 1 {
        lines[1..].to_vec()
    } else {
        vec!["fact".into()]
    };
    invoke_dual(
        document,
        label,
        "Research.trace_fiction",
        "trace_fiction sketch".into(),
        json!({ "fiction_content": fiction, "reality_corpus": corpus }),
    );
}

pub(super) fn run_assess_sentiment(document: &Document, label: &str) {
    run_content(document, label, "Research.assess_sentiment", "text");
}

pub(super) fn run_detect_sentiment_manipulation(document: &Document, label: &str) {
    let mut texts: Vec<String> = surface_text(document)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .take(16)
        .collect();
    if texts.is_empty() {
        texts = vec!["hello".into()];
    }
    invoke_dual(
        document,
        label,
        "Research.detect_sentiment_manipulation",
        "detect_sentiment_manipulation sketch".into(),
        json!({ "texts": texts }),
    );
}

pub(super) fn run_detect_performed_sentiment(document: &Document, label: &str) {
    run_content(document, label, "Research.detect_performed_sentiment", "text");
}

pub(super) fn run_map_sentiment_network(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Research.map_sentiment_network",
        "map_sentiment_network sketch".into(),
        json!({ "mentions": [["a", "b", 0.5]] }),
    );
}

pub(super) fn run_analyse_sentiment_trends(document: &Document, label: &str) {
    let mut values: Vec<f64> = surface_text(document)
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|'))
        .filter_map(|t| t.parse::<f64>().ok())
        .take(32)
        .collect();
    if values.is_empty() {
        values = vec![0.1, 0.2, 0.15];
    }
    invoke_dual(
        document,
        label,
        "Research.analyse_sentiment_trends",
        "analyse_sentiment_trends sketch".into(),
        json!({ "values": values }),
    );
}

pub(super) fn run_register_perspective(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.register_perspective",
        "register_perspective sketch",
        json!({ "agent_id": "poet", "viewpoint": content(document) }),
    );
}

pub(super) fn run_add_bias(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.add_bias",
        "add_bias sketch",
        json!({ "bias_type": "confirmation", "severity": 0.5 }),
    );
}

pub(super) fn run_compare_perspectives(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Research.compare_perspectives",
        "compare_perspectives sketch".into(),
        json!({ "viewpoint_a": "alpha", "viewpoint_b": content(document) }),
    );
}

pub(super) fn run_detect_perspective_conflict(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Research.detect_perspective_conflict",
        "detect_perspective_conflict sketch".into(),
        json!({ "viewpoint_a": "alpha", "viewpoint_b": content(document) }),
    );
}

pub(super) fn run_reconcile_perspectives(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Research.reconcile_perspectives",
        "reconcile_perspectives sketch".into(),
        json!({ "viewpoint_a": "alpha", "viewpoint_b": content(document) }),
    );
}

pub(super) fn run_assess_intentionality(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Research.assess_intentionality",
        "assess_intentionality sketch".into(),
        json!({
            "knew_outcome": false,
            "could_prevent": false,
            "repeated_behavior": false,
            "benefited": false,
        }),
    );
}

pub(super) fn run_classify_mistake(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Research.classify_mistake",
        "classify_mistake sketch".into(),
        json!({
            "first_occurrence": true,
            "pattern_matches": 0,
            "corrected_after_feedback": false,
            "systemic_factor": false,
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave29_research_defaults() {
        assert_eq!(DEFAULT_ID, "enquiry-1");
        assert_eq!(first_line("  x\n"), "x");
    }
}
