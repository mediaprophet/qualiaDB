//! Dual-path Tool Chest actions for Host-bound `Animation.*` leftovers.
//!
//! Spatial already Live-binds `Animation.evaluate_preset`. Remaining
//! family IDs use `animation:live_*` on chain `spatial:anim_live`.

use serde_json::json;
use web_sys::{Document, Element};

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
    let bounded: String = text.chars().take(4_096).collect();
    (!bounded.trim().is_empty()).then_some(bounded)
}

fn parse_f64s(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|'))
        .filter_map(|t| t.parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(16)
        .collect()
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
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

fn nums(document: &Document) -> Vec<f64> {
    parse_f64s(&selected_source(document).unwrap_or_default())
}

pub(super) fn run_spring_step(document: &Document, label: &str) {
    let n = nums(document);
    let current = n.first().copied().unwrap_or(0.0);
    let target = n.get(1).copied().unwrap_or(1.0);
    invoke_dual(
        document,
        label,
        "Animation.spring_step",
        format!("spring_step sketch {current}→{target}"),
        json!({ "current": current, "target": target, "velocity": 0.0 }),
    );
}

pub(super) fn run_sclerp_step(document: &Document, label: &str) {
    let t = numeric_attr(selected_container(document).as_ref(), "data-t").unwrap_or(0.5);
    invoke_dual(
        document,
        label,
        "Animation.sclerp_step",
        format!("sclerp_step sketch t={t} (identity motors)"),
        json!({ "t": t }),
    );
}

pub(super) fn run_squad_step(document: &Document, label: &str) {
    let t = numeric_attr(selected_container(document).as_ref(), "data-t").unwrap_or(0.5);
    invoke_dual(
        document,
        label,
        "Animation.squad_step",
        format!("squad_step sketch t={t} (identity quats)"),
        json!({ "t": t }),
    );
}

pub(super) fn run_list_presets(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Animation.list_presets",
        "list_presets sketch".to_string(),
        json!({}),
    );
}
