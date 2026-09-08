//! Dual-path Tool Chest actions for Host-bound CG leftovers (wave 24).

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

fn pair2(nums: &[f64], start: usize, fallback: [f64; 2]) -> [f64; 2] {
    if start + 1 < nums.len() {
        [nums[start], nums[start + 1]]
    } else {
        fallback
    }
}

fn points2(nums: &[f64], fallback: &[[f64; 2]]) -> Vec<serde_json::Value> {
    let mut pts = Vec::new();
    let mut i = 0;
    while i + 1 < nums.len() {
        pts.push(json!([nums[i], nums[i + 1]]));
        i += 2;
    }
    if pts.len() < 2 {
        fallback.iter().map(|p| json!([p[0], p[1]])).collect()
    } else {
        pts
    }
}

const SQUARE: [[f64; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
const TRI: [[f64; 2]; 3] = [[0.2, 0.2], [0.8, 0.2], [0.5, 0.8]];

pub(super) fn run_width_coreset(document: &Document, label: &str) {
    let pts = points2(&nums(document), &SQUARE);
    let epsilon = numeric_attr(selected_container(document).as_ref(), "data-epsilon").unwrap_or(0.25);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.width_coreset",
        format!("width_coreset sketch ε={epsilon} n={}", pts.len()),
        json!({ "points": pts, "epsilon": epsilon }),
    );
}

pub(super) fn run_dual_point_to_line(document: &Document, label: &str) {
    let point = pair2(&nums(document), 0, [1.0, 0.5]);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.dual_point_to_line",
        "dual_point_to_line sketch".to_string(),
        json!({ "point": point }),
    );
}

pub(super) fn run_dual_round_trip(document: &Document, label: &str) {
    let point = pair2(&nums(document), 0, [1.0, 0.5]);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.dual_round_trip",
        "dual_round_trip sketch".to_string(),
        json!({ "point": point }),
    );
}

pub(super) fn run_is_convex_polygon(document: &Document, label: &str) {
    let verts = points2(&nums(document), &SQUARE);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.is_convex_polygon",
        format!("is_convex_polygon sketch n={}", verts.len()),
        json!({ "vertices": verts }),
    );
}

pub(super) fn run_point_in_or_on_polygon(document: &Document, label: &str) {
    let n = nums(document);
    let point = pair2(&n, 0, [0.5, 0.5]);
    let polygon = points2(if n.len() > 2 { &n[2..] } else { &[] }, &SQUARE);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.point_in_or_on_polygon",
        format!("point_in_or_on_polygon sketch n={}", polygon.len()),
        json!({ "point": point, "polygon": polygon }),
    );
}

fn run_boolean(document: &Document, label: &str, cap: &'static str, sketch: &str) {
    let n = nums(document);
    let pts = points2(&n, &SQUARE);
    let (a, b) = if pts.len() >= 6 {
        let mid = pts.len() / 2;
        (pts[..mid].to_vec(), pts[mid..].to_vec())
    } else {
        (points2(&[], &SQUARE), points2(&[], &TRI))
    };
    invoke_dual(
        document,
        label,
        cap,
        format!("{sketch} |A|={} |B|={}", a.len(), b.len()),
        json!({ "a": a, "b": b }),
    );
}

pub(super) fn run_boolean_union_area(document: &Document, label: &str) {
    run_boolean(
        document,
        label,
        "ComputationalGeometry.boolean_union_area",
        "boolean_union_area sketch",
    );
}

pub(super) fn run_boolean_intersection_area(document: &Document, label: &str) {
    run_boolean(
        document,
        label,
        "ComputationalGeometry.boolean_intersection_area",
        "boolean_intersection_area sketch",
    );
}

pub(super) fn run_boolean_difference_area(document: &Document, label: &str) {
    run_boolean(
        document,
        label,
        "ComputationalGeometry.boolean_difference_area",
        "boolean_difference_area sketch",
    );
}
