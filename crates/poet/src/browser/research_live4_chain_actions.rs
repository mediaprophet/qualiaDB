//! Dual-path Tool Chest remainder for Host-bound `Research.*` ids (wave 29).
//!
//! Dynamics, grounding, and ungrounded-behaviour binds on `research:live`.

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

fn parse_f64s(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|'))
        .filter_map(|t| t.parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(32)
        .collect()
}

pub(super) fn run_define_social_dynamics(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.define_social_dynamics",
        "define_social_dynamics sketch",
        json!({ "network_type": "general" }),
    );
}

pub(super) fn run_define_economic_dynamics(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.define_economic_dynamics",
        "define_economic_dynamics sketch",
        json!({}),
    );
}

pub(super) fn run_define_spatiotemporal_dynamics(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.define_spatiotemporal_dynamics",
        "define_spatiotemporal_dynamics sketch",
        json!({ "diffusion_rate": 0.1 }),
    );
}

pub(super) fn run_analyse_social_network(document: &Document, label: &str) {
    let mut agents: Vec<String> = surface_text(document)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .take(16)
        .collect();
    if agents.is_empty() {
        agents = vec!["a".into(), "b".into()];
    }
    invoke_dual(
        document,
        label,
        "Research.analyse_social_network",
        "analyse_social_network sketch".into(),
        json!({ "agents": agents, "interactions": [] }),
    );
}

pub(super) fn run_analyse_inequality(document: &Document, label: &str) {
    let mut values = parse_f64s(&surface_text(document));
    if values.is_empty() {
        values = vec![1.0, 2.0, 4.0];
    }
    invoke_dual(
        document,
        label,
        "Research.analyse_inequality",
        "analyse_inequality sketch".into(),
        json!({ "values": values }),
    );
}

pub(super) fn run_analyse_diffusion(document: &Document, label: &str) {
    let mut initial = parse_f64s(&surface_text(document));
    if initial.is_empty() {
        initial = vec![1.0, 0.0, 0.0];
    }
    invoke_dual(
        document,
        label,
        "Research.analyse_diffusion",
        "analyse_diffusion sketch".into(),
        json!({ "initial_values": initial, "diffusion_rate": 0.1, "steps": 10 }),
    );
}

pub(super) fn run_assess_grounding(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Research.assess_grounding",
        "assess_grounding sketch".into(),
        json!({ "content": content(document) }),
    );
}

pub(super) fn run_verify_grounding(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Research.verify_grounding",
        "verify_grounding sketch".into(),
        json!({ "content": content(document) }),
    );
}

pub(super) fn run_detect_ungrounded_behaviour(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Research.detect_ungrounded_behaviour",
        "detect_ungrounded_behaviour sketch".into(),
        json!({ "content": content(document) }),
    );
}

pub(super) fn run_create_ug_instance(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.create_ug_instance",
        "create_ug_instance sketch",
        json!({}),
    );
}

pub(super) fn run_set_ug_cause(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.set_ug_cause",
        "set_ug_cause sketch",
        json!({ "cause": content(document) }),
    );
}

pub(super) fn run_set_ug_consequence(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.set_ug_consequence",
        "set_ug_consequence sketch",
        json!({ "consequence": content(document) }),
    );
}

pub(super) fn run_set_ug_detection(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.set_ug_detection",
        "set_ug_detection sketch",
        json!({ "detection": content(document) }),
    );
}

pub(super) fn run_set_ug_mitigation(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.set_ug_mitigation",
        "set_ug_mitigation sketch",
        json!({ "mitigation": content(document) }),
    );
}

pub(super) fn run_set_ug_calibration(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.set_ug_calibration",
        "set_ug_calibration sketch",
        json!({ "calibration": 0.5 }),
    );
}

pub(super) fn run_detect_ug_patterns(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.detect_ug_patterns",
        "detect_ug_patterns sketch",
        json!({}),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave29_research_dynamics_defaults() {
        assert_eq!(DEFAULT_ID, "enquiry-1");
        assert_eq!(parse_f64s("1 2,3"), vec![1.0, 2.0, 3.0]);
    }
}
