//! Dual-path Tool Chest actions for curated Host-bound `Portal.*` / `Avatar.*` ids (wave 26).

use serde_json::json;
use web_sys::{Document, Element};

const DEFAULT_PORTAL_ID: &str = "portal-1";
const DEFAULT_AVATAR_ID: &str = "avatar-1";
const DEFAULT_TARGET_WORLD: &str = "world-2";
const DEFAULT_MODEL: &str = "default";
const DEFAULT_HEIGHT: f64 = 1.8;
const DEFAULT_SCALE: f64 = 1.0;

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn string_attr(el: Option<&Element>, name: &str) -> Option<String> {
    el.and_then(|e| e.get_attribute(name))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
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

fn resolve_id(container: Option<&Element>, attr: &str, default: &str) -> String {
    string_attr(container, attr).unwrap_or_else(|| default.to_string())
}

fn portal_id(document: &Document) -> String {
    let el = selected_container(document);
    resolve_id(el.as_ref(), "data-portal-id", DEFAULT_PORTAL_ID)
}

/// `Portal.set_target` — `{ id, target_world }`.
pub(super) fn run_set_target(document: &Document, label: &str) {
    let el = selected_container(document);
    let id = resolve_id(el.as_ref(), "data-portal-id", DEFAULT_PORTAL_ID);
    let target = resolve_id(el.as_ref(), "data-target-world", DEFAULT_TARGET_WORLD);
    invoke_dual(
        document,
        label,
        "Portal.set_target",
        format!("portal set_target sketch id={id} target={target}"),
        json!({ "id": id, "target_world": target }),
    );
}

/// `Portal.activate` — `{ id }`.
pub(super) fn run_activate(document: &Document, label: &str) {
    let id = portal_id(document);
    invoke_dual(
        document,
        label,
        "Portal.activate",
        format!("portal activate sketch id={id}"),
        json!({ "id": id }),
    );
}

/// `Portal.deactivate` — `{ id }`.
pub(super) fn run_deactivate(document: &Document, label: &str) {
    let id = portal_id(document);
    invoke_dual(
        document,
        label,
        "Portal.deactivate",
        format!("portal deactivate sketch id={id}"),
        json!({ "id": id }),
    );
}

/// `Avatar.move` — `{ id, position: [f64;3] }`.
pub(super) fn run_move(document: &Document, label: &str) {
    let el = selected_container(document);
    let id = resolve_id(el.as_ref(), "data-avatar-id", DEFAULT_AVATAR_ID);
    let x = numeric_attr(el.as_ref(), "data-x").unwrap_or(0.0);
    let y = numeric_attr(el.as_ref(), "data-y").unwrap_or(0.0);
    let z = numeric_attr(el.as_ref(), "data-z").unwrap_or(0.0);
    invoke_dual(
        document,
        label,
        "Avatar.move",
        format!("avatar move sketch id={id} pos=[{x},{y},{z}]"),
        json!({ "id": id, "position": [x, y, z] }),
    );
}

/// `Avatar.set_appearance` — `{ id, model_id?, height?, scale? }`.
pub(super) fn run_set_appearance(document: &Document, label: &str) {
    let el = selected_container(document);
    let id = resolve_id(el.as_ref(), "data-avatar-id", DEFAULT_AVATAR_ID);
    let model = resolve_id(el.as_ref(), "data-model-id", DEFAULT_MODEL);
    let height = numeric_attr(el.as_ref(), "data-height").unwrap_or(DEFAULT_HEIGHT);
    let scale = numeric_attr(el.as_ref(), "data-scale").unwrap_or(DEFAULT_SCALE);
    invoke_dual(
        document,
        label,
        "Avatar.set_appearance",
        format!("avatar set_appearance sketch id={id} model={model}"),
        json!({ "id": id, "model_id": model, "height": height, "scale": scale }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_portal_wave26_defaults() {
        assert_eq!(DEFAULT_PORTAL_ID, "portal-1");
        assert_eq!(DEFAULT_AVATAR_ID, "avatar-1");
        assert_eq!(DEFAULT_HEIGHT, 1.8);
    }
}
