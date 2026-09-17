//! Dual-path Tool Chest actions for curated Host-bound `World.*` ids (wave 24).

use serde_json::json;
use web_sys::{Document, Element};

const DEFAULT_WORLD_ID: &str = "world-1";
const DEFAULT_WORLD_NAME: &str = "World";
const DEFAULT_OBJECT_NAME: &str = "Object";
const DEFAULT_TARGET: &str = "world-2";
const DEFAULT_DID: &str = "did:q42:user";
const DEFAULT_DT: f64 = 0.016;

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn string_attr(el: Option<&Element>, name: &str) -> Option<String> {
    el.and_then(|e| e.get_attribute(name))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
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
        super::interactions::show_tool_status(
            document,
            &label,
            &report.message,
            report.status_kind,
        );
        return;
    }
    super::interactions::show_tool_status(
        document,
        &label,
        &format!("Running {cap_id}…"),
        "running",
    );
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

fn local_id(from_attr: Option<&str>, default: &str) -> String {
    from_attr
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default)
        .to_string()
}

fn resolve_world_id(container: Option<&Element>) -> String {
    local_id(
        string_attr(container, "data-world-id").as_deref(),
        DEFAULT_WORLD_ID,
    )
}

fn local_gravity() -> [f64; 3] {
    [0.0, -9.81, 0.0]
}

fn local_force() -> [f64; 3] {
    [0.0, 0.0, 0.0]
}

/// `World.new` — `{ id, name? }`.
pub(super) fn run_new(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_world_id(container.as_ref());
    let name = local_id(
        string_attr(container.as_ref(), "data-name").as_deref(),
        DEFAULT_WORLD_NAME,
    );
    invoke_dual(
        document,
        label,
        "World.new",
        format!("world new sketch id={id} name={name} object_count=0"),
        json!({ "id": id, "name": name }),
    );
}

/// `World.add_object` — `{ id, name? }`.
pub(super) fn run_add_object(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_world_id(container.as_ref());
    let name = local_id(
        string_attr(container.as_ref(), "data-object-name").as_deref(),
        DEFAULT_OBJECT_NAME,
    );
    invoke_dual(
        document,
        label,
        "World.add_object",
        format!("world add_object sketch id={id} name={name}"),
        json!({ "id": id, "name": name }),
    );
}

/// `World.add_portal` — `{ id, target_world }`.
pub(super) fn run_add_portal(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_world_id(container.as_ref());
    let target_world = local_id(
        string_attr(container.as_ref(), "data-target-world").as_deref(),
        DEFAULT_TARGET,
    );
    invoke_dual(
        document,
        label,
        "World.add_portal",
        format!("world add_portal sketch id={id} target_world={target_world}"),
        json!({ "id": id, "target_world": target_world }),
    );
}

/// `World.add_avatar` — `{ id, user_did }`.
pub(super) fn run_add_avatar(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_world_id(container.as_ref());
    let user_did = local_id(
        string_attr(container.as_ref(), "data-user-did").as_deref(),
        DEFAULT_DID,
    );
    invoke_dual(
        document,
        label,
        "World.add_avatar",
        format!("world add_avatar sketch id={id} user_did={user_did}"),
        json!({ "id": id, "user_did": user_did }),
    );
}

/// `World.set_gravity` — `{ id, gravity? }`.
pub(super) fn run_set_gravity(document: &Document, label: &str) {
    let id = resolve_world_id(selected_container(document).as_ref());
    let gravity = local_gravity();
    invoke_dual(
        document,
        label,
        "World.set_gravity",
        format!("world set_gravity sketch id={id} gravity={gravity:?}"),
        json!({ "id": id, "gravity": gravity }),
    );
}

/// `World.object_apply_force` — `{ id, force, delta_time? }`.
pub(super) fn run_object_apply_force(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_world_id(container.as_ref());
    let force = local_force();
    let delta_time = numeric_attr(container.as_ref(), "data-dt").unwrap_or(DEFAULT_DT);
    invoke_dual(
        document,
        label,
        "World.object_apply_force",
        format!("world object_apply_force sketch id={id} force={force:?} dt={delta_time}"),
        json!({ "id": id, "force": force, "delta_time": delta_time }),
    );
}

/// `World.object_step_physics` — `{ id, delta_time? }`.
pub(super) fn run_object_step_physics(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_world_id(container.as_ref());
    let delta_time = numeric_attr(container.as_ref(), "data-dt").unwrap_or(DEFAULT_DT);
    invoke_dual(
        document,
        label,
        "World.object_step_physics",
        format!("world object_step_physics sketch id={id} dt={delta_time}"),
        json!({ "id": id, "delta_time": delta_time }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_world_wave24_defaults() {
        assert_eq!(local_id(None, "world-1"), "world-1");
        assert_eq!(local_gravity(), [0.0, -9.81, 0.0]);
        assert_eq!(local_force(), [0.0, 0.0, 0.0]);
    }
}
