//! Dual-path Tool Chest actions for Host-bound CG leftovers (waves 19–22).
//!
//! Existing `scientific:cg_*` ids occupy the first CG chain. New Live tools
//! use `scientific:cg_live_*` on chain `scientific:cg_live`.

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

fn k_attr(document: &Document, default: u64) -> u64 {
    numeric_attr(selected_container(document).as_ref(), "data-k")
        .and_then(|v| (v.is_finite() && v >= 1.0).then_some(v as u64))
        .unwrap_or(default)
}

fn points3(nums: &[f64], fallback: &[[f64; 3]]) -> Vec<serde_json::Value> {
    let mut pts = Vec::new();
    let mut i = 0;
    while i + 2 < nums.len() {
        pts.push(json!([nums[i], nums[i + 1], nums[i + 2]]));
        i += 3;
    }
    if pts.len() < 2 {
        fallback.iter().map(|p| json!([p[0], p[1], p[2]])).collect()
    } else {
        pts
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

fn pair2(nums: &[f64], start: usize, fallback: [f64; 2]) -> [f64; 2] {
    if start + 1 < nums.len() {
        [nums[start], nums[start + 1]]
    } else {
        fallback
    }
}

const TETRA: [[f64; 3]; 4] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
];
const SQUARE: [[f64; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

fn run_points3_k(document: &Document, label: &str, cap: &'static str, sketch: &str) {
    let pts = points3(&nums(document), &TETRA);
    let k = k_attr(document, 1);
    invoke_dual(
        document,
        label,
        cap,
        format!("{sketch} k={k} n={}", pts.len()),
        json!({ "points": pts, "k": k }),
    );
}

fn run_pq(document: &Document, label: &str, cap: &'static str, sketch: &str) {
    let n = nums(document);
    let (p, q) = if n.len() >= 4 {
        let mid = n.len() / 2;
        (n[..mid].to_vec(), n[mid..].to_vec())
    } else {
        (vec![0.5, 0.5], vec![0.25, 0.75])
    };
    invoke_dual(
        document,
        label,
        cap,
        format!("{sketch} dim={}", p.len().min(q.len())),
        json!({ "p": p, "q": q }),
    );
}

pub(super) fn run_average_spacing_3d(document: &Document, label: &str) {
    run_points3_k(
        document,
        label,
        "ComputationalGeometry.average_spacing_3d",
        "average_spacing_3d sketch",
    );
}

pub(super) fn run_local_density_3d(document: &Document, label: &str) {
    run_points3_k(
        document,
        label,
        "ComputationalGeometry.local_density_3d",
        "local_density_3d sketch",
    );
}

pub(super) fn run_mean_knn_distance_3d(document: &Document, label: &str) {
    run_points3_k(
        document,
        label,
        "ComputationalGeometry.mean_knn_distance_3d",
        "mean_knn_distance_3d sketch",
    );
}

pub(super) fn run_fisher_distance(document: &Document, label: &str) {
    run_pq(
        document,
        label,
        "ComputationalGeometry.fisher_distance",
        "fisher_distance sketch",
    );
}

pub(super) fn run_kl_divergence(document: &Document, label: &str) {
    run_pq(
        document,
        label,
        "ComputationalGeometry.kl_divergence",
        "kl_divergence sketch",
    );
}

pub(super) fn run_kl_bregman_form(document: &Document, label: &str) {
    run_pq(
        document,
        label,
        "ComputationalGeometry.kl_bregman_form",
        "kl_bregman_form sketch",
    );
}

pub(super) fn run_triangle_signed_area(document: &Document, label: &str) {
    let n = nums(document);
    let a = pair2(&n, 0, [0.0, 0.0]);
    let b = pair2(&n, 2, [1.0, 0.0]);
    let c = pair2(&n, 4, [0.0, 1.0]);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.triangle_signed_area",
        "triangle_signed_area sketch (unit right triangle)",
        json!({ "a": a, "b": b, "c": c }),
    );
}

fn run_dist_seg(document: &Document, label: &str, cap: &'static str, sketch: &str) {
    let n = nums(document);
    let point = pair2(&n, 0, [0.5, 0.5]);
    let a = pair2(&n, 2, [0.0, 0.0]);
    let b = pair2(&n, 4, [1.0, 0.0]);
    invoke_dual(
        document,
        label,
        cap,
        sketch.to_string(),
        json!({ "point": point, "a": a, "b": b }),
    );
}

pub(super) fn run_dist_point_to_segment(document: &Document, label: &str) {
    run_dist_seg(
        document,
        label,
        "ComputationalGeometry.dist_point_to_segment",
        "dist_point_to_segment sketch",
    );
}

pub(super) fn run_dist_sq_point_to_segment(document: &Document, label: &str) {
    run_dist_seg(
        document,
        label,
        "ComputationalGeometry.dist_sq_point_to_segment",
        "dist_sq_point_to_segment sketch",
    );
}

pub(super) fn run_incircle(document: &Document, label: &str) {
    let n = nums(document);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.incircle",
        "incircle sketch (unit triangle vs origin)",
        json!({
            "a": pair2(&n, 0, [0.0, 0.0]),
            "b": pair2(&n, 2, [1.0, 0.0]),
            "c": pair2(&n, 4, [0.0, 1.0]),
            "d": pair2(&n, 6, [0.2, 0.2]),
        }),
    );
}

pub(super) fn run_tukey_depth(document: &Document, label: &str) {
    let n = nums(document);
    let query = pair2(&n, 0, [0.5, 0.5]);
    let pts = if n.len() >= 6 {
        points2(&n[2.min(n.len())..], &SQUARE)
    } else {
        points2(&[], &SQUARE)
    };
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.tukey_depth",
        format!("tukey_depth sketch n={}", pts.len()),
        json!({ "query": query, "points": pts }),
    );
}

pub(super) fn run_directional_width(document: &Document, label: &str) {
    let n = nums(document);
    let pts = points2(&n, &SQUARE);
    let dir = pair2(&n, n.len().saturating_sub(2), [1.0, 0.0]);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.directional_width",
        format!("directional_width sketch n={}", pts.len()),
        json!({ "points": pts, "dir": dir }),
    );
}

pub(super) fn run_width(document: &Document, label: &str) {
    let pts = points2(&nums(document), &SQUARE);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.width",
        format!("width sketch n={}", pts.len()),
        json!({ "points": pts }),
    );
}

pub(super) fn run_farthest_site_brute(document: &Document, label: &str) {
    let n = nums(document);
    let q = pair2(&n, 0, [0.5, 0.5]);
    let sites = points2(if n.len() > 2 { &n[2..] } else { &[] }, &SQUARE);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.farthest_site_brute",
        format!("farthest_site_brute sketch n={}", sites.len()),
        json!({ "sites": sites, "q": q }),
    );
}

pub(super) fn run_k_nearest_sites(document: &Document, label: &str) {
    let n = nums(document);
    let q = pair2(&n, 0, [0.5, 0.5]);
    let sites = points2(if n.len() > 2 { &n[2..] } else { &[] }, &SQUARE);
    let k = k_attr(document, 1);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.k_nearest_sites",
        format!("k_nearest_sites sketch k={k} n={}", sites.len()),
        json!({ "sites": sites, "q": q, "k": k }),
    );
}

pub(super) fn run_is_hull_site(document: &Document, label: &str) {
    let n = nums(document);
    let sites = points2(&n, &SQUARE);
    let index = numeric_attr(selected_container(document).as_ref(), "data-index")
        .and_then(|v| (v >= 0.0).then_some(v as u64))
        .unwrap_or(0);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.is_hull_site",
        format!("is_hull_site sketch index={index} n={}", sites.len()),
        json!({ "sites": sites, "index": index }),
    );
}

pub(super) fn run_diameter_and_width(document: &Document, label: &str) {
    let pts = points2(&nums(document), &SQUARE);
    invoke_dual(
        document,
        label,
        "ComputationalGeometry.diameter_and_width",
        format!("diameter_and_width sketch n={}", pts.len()),
        json!({ "points": pts }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave31_points3_falls_back_to_tetra() {
        let pts = points3(&[], &TETRA);
        assert_eq!(pts.len(), 4);
    }
}
