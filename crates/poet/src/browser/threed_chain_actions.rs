//! Dual-path Tool Chest actions for curated `ThreeD.*` ALL_BOUND ids (wave 20).
//!
//! Prefers numeric surface fields (position / fov / duration) where Host accepts them.
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.

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
    let bounded: String = text.chars().take(16_384).collect();
    (!bounded.trim().is_empty()).then_some(bounded)
}

fn parse_numbers(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|' | '\n' | '\r'))
        .filter_map(|token| token.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(64)
        .collect()
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
}

fn string_attr(el: Option<&Element>, name: &str) -> Option<String> {
    el.and_then(|e| e.get_attribute(name))
        .filter(|s| !s.trim().is_empty())
}

fn first_token(source: &str) -> Option<String> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|'))
        .map(str::trim)
        .find(|t| !t.is_empty() && t.parse::<f64>().is_err())
        .map(|t| t.chars().take(64).collect())
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

fn resolve_id(container: Option<&Element>, source: &str, fallback: &str) -> String {
    string_attr(container, "data-id")
        .or_else(|| first_token(source))
        .unwrap_or_else(|| fallback.into())
}

fn resolve_position(container: Option<&Element>, nums: &[f64]) -> [f64; 3] {
    let x = numeric_attr(container, "data-x")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.0);
    let y = numeric_attr(container, "data-y")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(0.0);
    let z = numeric_attr(container, "data-z")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(0.0);
    [x, y, z]
}

/// `ThreeD.add_object` — `{ id, name? }`.
pub(super) fn run_add_object(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let id = resolve_id(container.as_ref(), &source, "obj-1");
    let name = string_attr(container.as_ref(), "data-name")
        .or_else(|| {
            source
                .split_whitespace()
                .nth(1)
                .map(|s| s.chars().take(64).collect())
        })
        .unwrap_or_else(|| "Object".into());
    invoke_dual(
        document,
        label,
        "ThreeD.add_object",
        format!("ThreeD object sketch id={id} name={name}"),
        json!({ "id": id, "name": name }),
    );
}

/// `ThreeD.set_transform` — `{ id, position }` (numeric-leaning).
pub(super) fn run_set_transform(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let id = resolve_id(container.as_ref(), &source, "obj-1");
    let pos = resolve_position(container.as_ref(), &nums);
    invoke_dual(
        document,
        label,
        "ThreeD.set_transform",
        format!(
            "ThreeD transform sketch id={id} pos=[{:.3}, {:.3}, {:.3}]",
            pos[0], pos[1], pos[2]
        ),
        json!({ "id": id, "position": pos }),
    );
}

/// `ThreeD.set_material` — `{ id, material_id }`.
pub(super) fn run_set_material(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let id = resolve_id(container.as_ref(), &source, "obj-1");
    let material_id = string_attr(container.as_ref(), "data-material-id")
        .or_else(|| {
            source
                .split_whitespace()
                .nth(1)
                .map(|s| s.chars().take(64).collect())
        })
        .unwrap_or_else(|| "mat-default".into());
    invoke_dual(
        document,
        label,
        "ThreeD.set_material",
        format!("ThreeD material sketch id={id} material={material_id}"),
        json!({ "id": id, "material_id": material_id }),
    );
}

/// `ThreeD.add_camera` — `{ id, fov? }` (numeric fov).
pub(super) fn run_add_camera(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let id = resolve_id(container.as_ref(), &source, "cam-1");
    let fov = numeric_attr(container.as_ref(), "data-fov")
        .or_else(|| nums.first().copied())
        .unwrap_or(60.0);
    invoke_dual(
        document,
        label,
        "ThreeD.add_camera",
        format!("ThreeD camera sketch id={id} fov={fov:.1}"),
        json!({ "id": id, "fov": fov }),
    );
}

/// `ThreeD.add_light` — `{ id, light_type? }`.
pub(super) fn run_add_light(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let id = resolve_id(container.as_ref(), &source, "light-1");
    let light_type = string_attr(container.as_ref(), "data-light-type")
        .or_else(|| {
            source
                .split_whitespace()
                .find(|t| {
                    matches!(
                        t.to_ascii_lowercase().as_str(),
                        "point" | "directional" | "spot" | "ambient"
                    )
                })
                .map(|s| s.to_ascii_lowercase())
        })
        .unwrap_or_else(|| "point".into());
    invoke_dual(
        document,
        label,
        "ThreeD.add_light",
        format!("ThreeD light sketch id={id} type={light_type}"),
        json!({ "id": id, "light_type": light_type }),
    );
}

/// `ThreeD.add_rig` — `{ id, name? }`.
pub(super) fn run_add_rig(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let id = resolve_id(container.as_ref(), &source, "rig-1");
    let name = string_attr(container.as_ref(), "data-name")
        .or_else(|| {
            source
                .split_whitespace()
                .nth(1)
                .map(|s| s.chars().take(64).collect())
        })
        .unwrap_or_else(|| "Rig".into());
    invoke_dual(
        document,
        label,
        "ThreeD.add_rig",
        format!("ThreeD rig sketch id={id} name={name}"),
        json!({ "id": id, "name": name }),
    );
}

/// `ThreeD.add_animation` — `{ id, duration? }` (numeric duration).
pub(super) fn run_add_animation(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let id = resolve_id(container.as_ref(), &source, "anim-1");
    let duration = numeric_attr(container.as_ref(), "data-duration")
        .or_else(|| nums.first().copied())
        .unwrap_or(1.0);
    invoke_dual(
        document,
        label,
        "ThreeD.add_animation",
        format!("ThreeD animation sketch id={id} duration={duration:.3}s"),
        json!({ "id": id, "duration": duration }),
    );
}

/// `ThreeD.set_mesh` — `{ id, mesh_id }`.
pub(super) fn run_set_mesh(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let id = resolve_id(container.as_ref(), &source, "obj-1");
    let mesh_id = string_attr(container.as_ref(), "data-mesh-id")
        .or_else(|| {
            source
                .split_whitespace()
                .nth(1)
                .map(|s| s.chars().take(64).collect())
        })
        .unwrap_or_else(|| "mesh-default".into());
    invoke_dual(
        document,
        label,
        "ThreeD.set_mesh",
        format!("ThreeD mesh sketch id={id} mesh={mesh_id}"),
        json!({ "id": id, "mesh_id": mesh_id }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_threed_wave20_numeric_defaults() {
        let pos = resolve_position(None, &[1.0, 2.0, 3.0]);
        assert_eq!(pos, [1.0, 2.0, 3.0]);
        let pos0 = resolve_position(None, &[]);
        assert_eq!(pos0, [0.0, 0.0, 0.0]);
        assert_eq!(resolve_id(None, "", "obj-1"), "obj-1");
        assert_eq!(resolve_id(None, "camA 45", "cam-1"), "camA");
        let nums = parse_numbers("idX 90.5");
        assert_eq!(nums, vec![90.5]);
    }
}
