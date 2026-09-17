//! Dual-path Tool Chest actions for Host-bound CG leftovers (waves 25–26).

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
    let bounded: String = text.chars().take(8_192).collect();
    (!bounded.trim().is_empty()).then_some(bounded)
}

fn parse_f64s(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|'))
        .filter_map(|t| t.parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(64)
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

fn n_at(nums: &[f64], i: usize, fallback: f64) -> f64 {
    nums.get(i).copied().unwrap_or(fallback)
}

fn triple(nums: &[f64], start: usize, fallback: [f64; 3]) -> [f64; 3] {
    if start + 2 < nums.len() {
        [nums[start], nums[start + 1], nums[start + 2]]
    } else {
        fallback
    }
}

pub(super) fn run_cross_ratio_1d(document: &Document, label: &str) {
    let n = nums(document);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.cross_ratio_1d",
        "cross_ratio_1d sketch".to_string(),
        json!({
            "a": n_at(&n, 0, 0.0),
            "b": n_at(&n, 1, 1.0),
            "c": n_at(&n, 2, 2.0),
            "d": n_at(&n, 3, 3.0),
        }),
    );
}

pub(super) fn run_hyperplane_eval(document: &Document, label: &str) {
    let n = nums(document);
    let offset = numeric_attr(selected_container(document).as_ref(), "data-offset").unwrap_or(0.0);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.hyperplane_eval",
        "hyperplane_eval sketch".to_string(),
        json!({
            "normal": triple(&n, 0, [0.0, 0.0, 1.0]),
            "point": triple(&n, 3, [0.0, 0.0, 1.0]),
            "offset": offset,
        }),
    );
}

pub(super) fn run_householder_reflect(document: &Document, label: &str) {
    let n = nums(document);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.householder_reflect",
        "householder_reflect sketch".to_string(),
        json!({
            "v": triple(&n, 0, [1.0, 0.0, 0.0]),
            "normal": triple(&n, 3, [0.0, 0.0, 1.0]),
        }),
    );
}

fn run_quat(document: &Document, label: &str, cap: &'static str, sketch: &str) {
    let n = nums(document);
    invoke_dual(
        document,
        label,
        cap,
        sketch.to_string(),
        json!({
            "w": n_at(&n, 0, 1.0),
            "x": n_at(&n, 1, 0.0),
            "y": n_at(&n, 2, 0.0),
            "z": n_at(&n, 3, 0.0),
        }),
    );
}

pub(super) fn run_quaternion_normalize(document: &Document, label: &str) {
    run_quat(
        document,
        label,
        "ComputationalGeometry.quaternion_normalize",
        "quaternion_normalize sketch",
    );
}

pub(super) fn run_so3_exp(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.so3_exp",
        "so3_exp sketch".to_string(),
        json!({ "axis_angle": triple(&nums(document), 0, [0.0, 0.0, 0.5]) }),
    );
}

pub(super) fn run_so3_log(document: &Document, label: &str) {
    run_quat(
        document,
        label,
        "ComputationalGeometry.so3_log",
        "so3_log sketch",
    );
}

pub(super) fn run_projective_from_point(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.projective_from_point",
        "projective_from_point sketch".to_string(),
        json!({ "point": triple(&nums(document), 0, [1.0, 2.0, 3.0]) }),
    );
}

pub(super) fn run_point_from_projective(document: &Document, label: &str) {
    let n = nums(document);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.point_from_projective",
        "point_from_projective sketch".to_string(),
        json!({
            "x": n_at(&n, 0, 1.0),
            "y": n_at(&n, 1, 2.0),
            "z": n_at(&n, 2, 3.0),
            "w": n_at(&n, 3, 1.0),
        }),
    );
}

pub(super) fn run_frame_to_world(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.frame_to_world",
        "frame_to_world sketch (identity frame)".to_string(),
        json!({ "coords": triple(&nums(document), 0, [1.0, 2.0, 3.0]) }),
    );
}

pub(super) fn run_world_to_frame(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.world_to_frame",
        "world_to_frame sketch (identity frame)".to_string(),
        json!({ "point": triple(&nums(document), 0, [1.0, 2.0, 3.0]) }),
    );
}

pub(super) fn run_barycentric_tetra(document: &Document, label: &str) {
    let n = nums(document);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.barycentric_tetra",
        "barycentric_tetra sketch".to_string(),
        json!({
            "p": triple(&n, 0, [0.25, 0.25, 0.25]),
            "a": triple(&n, 3, [0.0, 0.0, 0.0]),
            "b": triple(&n, 6, [1.0, 0.0, 0.0]),
            "c": triple(&n, 9, [0.0, 1.0, 0.0]),
            "d": triple(&n, 12, [0.0, 0.0, 1.0]),
        }),
    );
}

pub(super) fn run_quaternion_slerp(document: &Document, label: &str) {
    let n = nums(document);
    let t = numeric_attr(selected_container(document).as_ref(), "data-t").unwrap_or(0.5);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.quaternion_slerp",
        format!("quaternion_slerp sketch t={t}"),
        json!({
            "a_w": n_at(&n, 0, 1.0),
            "a_x": n_at(&n, 1, 0.0),
            "a_y": n_at(&n, 2, 0.0),
            "a_z": n_at(&n, 3, 0.0),
            "b_w": n_at(&n, 4, 0.7071),
            "b_x": n_at(&n, 5, 0.0),
            "b_y": n_at(&n, 6, 0.0),
            "b_z": n_at(&n, 7, 0.7071),
            "t": t,
        }),
    );
}

pub(super) fn run_quaternion_to_matrix(document: &Document, label: &str) {
    run_quat(
        document,
        label,
        "ComputationalGeometry.quaternion_to_matrix",
        "quaternion_to_matrix sketch",
    );
}

pub(super) fn run_solve_diagonal_quadratic(document: &Document, label: &str) {
    let n = nums(document);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.solve_diagonal_quadratic",
        "solve_diagonal_quadratic sketch".to_string(),
        json!({
            "diag": triple(&n, 0, [1.0, 2.0, 3.0]),
            "linear": triple(&n, 3, [0.0, 0.0, 0.0]),
        }),
    );
}

pub(super) fn run_schur_complement_2x2(document: &Document, label: &str) {
    let n = nums(document);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.schur_complement_2x2",
        "schur_complement_2x2 sketch".to_string(),
        json!({
            "a00": n_at(&n, 0, 2.0),
            "a01": n_at(&n, 1, 0.0),
            "a11": n_at(&n, 2, 2.0),
            "b": [n_at(&n, 3, 1.0), n_at(&n, 4, 0.0)],
        }),
    );
}

pub(super) fn run_separating_plane_aabb(document: &Document, label: &str) {
    let n = nums(document);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.separating_plane_aabb",
        "separating_plane_aabb sketch".to_string(),
        json!({
            "a_min": triple(&n, 0, [0.0, 0.0, 0.0]),
            "a_max": triple(&n, 3, [1.0, 1.0, 1.0]),
            "b_min": triple(&n, 6, [2.0, 0.0, 0.0]),
            "b_max": triple(&n, 9, [3.0, 1.0, 1.0]),
        }),
    );
}
