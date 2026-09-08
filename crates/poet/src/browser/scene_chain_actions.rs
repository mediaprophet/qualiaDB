//! Dual-path Tool Chest actions for curated `Scene.*` ALL_BOUND ids (wave 21).
//!
//! Prefers pure numeric surfaces (camera lerp / frame, smooth damp, IK, budget, clear).
//! Includes wave-20 Host `Scene.lerp_camera` / `Scene.camera_frame_node`.
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.

use serde_json::{json, Value};
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

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: Value,
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

fn cam_record(yaw: f64, pitch: f64, zoom: f64) -> Value {
    json!({ "yaw": yaw, "pitch": pitch, "zoom": zoom })
}

fn resolve_camera_pair(container: Option<&Element>, nums: &[f64]) -> (Value, Value, f64) {
    let a = cam_record(
        numeric_attr(container, "data-yaw-a")
            .or_else(|| nums.first().copied())
            .unwrap_or(0.0),
        numeric_attr(container, "data-pitch-a")
            .or_else(|| nums.get(1).copied())
            .unwrap_or(0.0),
        numeric_attr(container, "data-zoom-a")
            .or_else(|| nums.get(2).copied())
            .unwrap_or(1.0),
    );
    let b = cam_record(
        numeric_attr(container, "data-yaw-b")
            .or_else(|| nums.get(3).copied())
            .unwrap_or(1.5),
        numeric_attr(container, "data-pitch-b")
            .or_else(|| nums.get(4).copied())
            .unwrap_or(0.2),
        numeric_attr(container, "data-zoom-b")
            .or_else(|| nums.get(5).copied())
            .unwrap_or(4.0),
    );
    let t = numeric_attr(container, "data-t")
        .or_else(|| nums.get(6).copied())
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    (a, b, t)
}

fn resolve_vec3(nums: &[f64], offset: usize, fallback: [f64; 3]) -> [f64; 3] {
    [
        nums.get(offset).copied().unwrap_or(fallback[0]),
        nums.get(offset + 1).copied().unwrap_or(fallback[1]),
        nums.get(offset + 2).copied().unwrap_or(fallback[2]),
    ]
}

fn resolve_node(container: Option<&Element>, nums: &[f64]) -> [f64; 3] {
    [
        numeric_attr(container, "data-x")
            .or_else(|| nums.first().copied())
            .unwrap_or(1.0),
        numeric_attr(container, "data-y")
            .or_else(|| nums.get(1).copied())
            .unwrap_or(0.5),
        numeric_attr(container, "data-z")
            .or_else(|| nums.get(2).copied())
            .unwrap_or(-2.0),
    ]
}

fn joints_from_nums(nums: &[f64]) -> Vec<[f64; 3]> {
    let mut joints = Vec::new();
    let mut i = 0;
    while i + 2 < nums.len() && joints.len() < 8 {
        joints.push([nums[i], nums[i + 1], nums[i + 2]]);
        i += 3;
    }
    if joints.len() < 2 {
        vec![[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 2.0, 0.0]]
    } else {
        joints
    }
}

fn joints_json(joints: &[[f64; 3]]) -> Value {
    Value::Array(
        joints
            .iter()
            .map(|p| json!([p[0], p[1], p[2]]))
            .collect(),
    )
}

/// `Scene.lerp_camera` — `{ a, b, t }` orbit cameras.
pub(super) fn run_lerp_camera(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (a, b, t) = resolve_camera_pair(container.as_ref(), &nums);
    invoke_dual(
        document,
        label,
        "Scene.lerp_camera",
        format!("Scene camera lerp sketch t={t:.3}"),
        json!({ "a": a, "b": b, "t": t }),
    );
}

/// `Scene.camera_frame_node` — `{ node: [x,y,z] }`.
pub(super) fn run_camera_frame_node(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let node = resolve_node(container.as_ref(), &nums);
    invoke_dual(
        document,
        label,
        "Scene.camera_frame_node",
        format!(
            "Scene frame-node sketch node=[{:.3}, {:.3}, {:.3}]",
            node[0], node[1], node[2]
        ),
        json!({ "node": node }),
    );
}

/// `Scene.smooth_damp` — scalar damp toward target.
pub(super) fn run_smooth_damp(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let current = numeric_attr(container.as_ref(), "data-current")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.0);
    let target = numeric_attr(container.as_ref(), "data-target")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(1.0);
    let velocity = numeric_attr(container.as_ref(), "data-velocity")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(0.0);
    let smooth_time = numeric_attr(container.as_ref(), "data-smooth-time")
        .or_else(|| nums.get(3).copied())
        .unwrap_or(0.3);
    let delta_time = numeric_attr(container.as_ref(), "data-delta-time")
        .or_else(|| nums.get(4).copied())
        .unwrap_or(0.016);
    invoke_dual(
        document,
        label,
        "Scene.smooth_damp",
        format!("Scene smooth_damp sketch {current:.3}→{target:.3}"),
        json!({
            "current": current,
            "target": target,
            "velocity": velocity,
            "smooth_time": smooth_time,
            "delta_time": delta_time,
        }),
    );
}

/// `Scene.smooth_damp_vec3` — vector damp toward target.
pub(super) fn run_smooth_damp_vec3(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let current = [
        numeric_attr(container.as_ref(), "data-x")
            .or_else(|| nums.first().copied())
            .unwrap_or(0.0),
        numeric_attr(container.as_ref(), "data-y")
            .or_else(|| nums.get(1).copied())
            .unwrap_or(0.0),
        numeric_attr(container.as_ref(), "data-z")
            .or_else(|| nums.get(2).copied())
            .unwrap_or(0.0),
    ];
    let target = resolve_vec3(&nums, 3, [1.0, 1.0, 0.0]);
    let velocity = resolve_vec3(&nums, 6, [0.0, 0.0, 0.0]);
    let smooth_time = numeric_attr(container.as_ref(), "data-smooth-time")
        .or_else(|| nums.get(9).copied())
        .unwrap_or(0.3);
    let delta_time = numeric_attr(container.as_ref(), "data-delta-time")
        .or_else(|| nums.get(10).copied())
        .unwrap_or(0.016);
    invoke_dual(
        document,
        label,
        "Scene.smooth_damp_vec3",
        format!(
            "Scene smooth_damp_vec3 sketch [{:.2},{:.2},{:.2}]→[{:.2},{:.2},{:.2}]",
            current[0], current[1], current[2], target[0], target[1], target[2]
        ),
        json!({
            "current": current,
            "target": target,
            "velocity": velocity,
            "smooth_time": smooth_time,
            "delta_time": delta_time,
        }),
    );
}

/// `Scene.ik_look_at` — `{ joints, target }`.
pub(super) fn run_ik_look_at(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let joints = joints_from_nums(&nums);
    let target = if nums.len() >= joints.len() * 3 + 3 {
        let o = joints.len() * 3;
        [nums[o], nums[o + 1], nums[o + 2]]
    } else {
        [
            numeric_attr(container.as_ref(), "data-target-x").unwrap_or(1.0),
            numeric_attr(container.as_ref(), "data-target-y").unwrap_or(2.0),
            numeric_attr(container.as_ref(), "data-target-z").unwrap_or(0.0),
        ]
    };
    invoke_dual(
        document,
        label,
        "Scene.ik_look_at",
        format!(
            "Scene ik_look_at sketch joints={} target=[{:.2},{:.2},{:.2}]",
            joints.len(),
            target[0],
            target[1],
            target[2]
        ),
        json!({ "joints": joints_json(&joints), "target": target }),
    );
}

/// `Scene.ik_ccd` — `{ joints, target, max_iterations?, tolerance? }`.
pub(super) fn run_ik_ccd(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let joints = joints_from_nums(&nums);
    let base = joints.len() * 3;
    let target = if nums.len() >= base + 3 {
        [nums[base], nums[base + 1], nums[base + 2]]
    } else {
        [
            numeric_attr(container.as_ref(), "data-target-x").unwrap_or(1.0),
            numeric_attr(container.as_ref(), "data-target-y").unwrap_or(2.0),
            numeric_attr(container.as_ref(), "data-target-z").unwrap_or(0.0),
        ]
    };
    let max_iterations = numeric_attr(container.as_ref(), "data-max-iterations")
        .or_else(|| nums.get(base + 3).copied())
        .unwrap_or(50.0)
        .round()
        .clamp(1.0, 500.0) as u64;
    let tolerance = numeric_attr(container.as_ref(), "data-tolerance")
        .or_else(|| nums.get(base + 4).copied())
        .unwrap_or(0.01);
    invoke_dual(
        document,
        label,
        "Scene.ik_ccd",
        format!(
            "Scene ik_ccd sketch joints={} max_iter={max_iterations} tol={tolerance:.4}",
            joints.len()
        ),
        json!({
            "joints": joints_json(&joints),
            "target": target,
            "max_iterations": max_iterations,
            "tolerance": tolerance,
        }),
    );
}

/// `Scene.set_render_budget` — `{ budget_ms }`.
pub(super) fn run_set_render_budget(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let budget_ms = numeric_attr(container.as_ref(), "data-budget-ms")
        .or_else(|| nums.first().copied())
        .unwrap_or(16.0)
        .max(0.1);
    invoke_dual(
        document,
        label,
        "Scene.set_render_budget",
        format!("Scene render budget sketch {budget_ms:.2} ms"),
        json!({ "budget_ms": budget_ms }),
    );
}

/// `Scene.set_clear_colour` — `{ r, g, b, a }`.
pub(super) fn run_set_clear_colour(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let r = numeric_attr(container.as_ref(), "data-r")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.05);
    let g = numeric_attr(container.as_ref(), "data-g")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(0.05);
    let b = numeric_attr(container.as_ref(), "data-b")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(0.08);
    let a = numeric_attr(container.as_ref(), "data-a")
        .or_else(|| nums.get(3).copied())
        .unwrap_or(1.0);
    invoke_dual(
        document,
        label,
        "Scene.set_clear_colour",
        format!("Scene clear colour sketch rgba=({r:.3},{g:.3},{b:.3},{a:.3})"),
        json!({ "r": r, "g": g, "b": b, "a": a }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_scene_wave21_numeric_defaults() {
        let (a, b, t) = resolve_camera_pair(None, &[]);
        assert_eq!(a["yaw"], json!(0.0));
        assert_eq!(b["zoom"], json!(4.0));
        assert!((t - 0.5).abs() < 1e-9);
        let node = resolve_node(None, &[2.0, 3.0, 4.0]);
        assert_eq!(node, [2.0, 3.0, 4.0]);
        let joints = joints_from_nums(&[]);
        assert_eq!(joints.len(), 3);
        let joints2 = joints_from_nums(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_eq!(joints2.len(), 2);
        let nums = parse_numbers("0.1 0.2 0.3 1.0");
        assert_eq!(nums, vec![0.1, 0.2, 0.3, 1.0]);
    }
}
