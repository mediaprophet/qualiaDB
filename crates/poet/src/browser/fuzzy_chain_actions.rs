//! Dual-path Tool Chest actions for curated `FuzzyQuery.*` ALL_BOUND ids (wave 18).
//!
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.
//! Local sketches mirror Host membership / degree algebra (CPU; Host path authoritative).

use serde_json::json;
use web_sys::{Document, Element};

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn need_container(document: &Document, label: &str, message: &str) -> Option<Element> {
    match selected_container(document) {
        Some(container) => Some(container),
        None => {
            super::interactions::show_tool_status(document, label, message, "error");
            None
        }
    }
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn surface_f64(container: &Element, name: &str, default: f64) -> f64 {
    numeric_attr(Some(container), name).unwrap_or(default)
}

fn surface_u64(container: &Element, name: &str, default: u64) -> u64 {
    numeric_attr(Some(container), name)
        .filter(|v| *v >= 0.0 && *v == v.floor())
        .map(|v| v as u64)
        .unwrap_or(default)
}

fn surface_norm(container: &Element) -> String {
    container
        .get_attribute("data-norm")
        .filter(|s| {
            matches!(
                s.trim().to_ascii_lowercase().as_str(),
                "godel" | "gödel" | "product" | "lukasiewicz"
            )
        })
        .map(|s| {
            let lower = s.trim().to_ascii_lowercase();
            if lower == "gödel" {
                "godel".into()
            } else {
                lower
            }
        })
        .unwrap_or_else(|| "godel".into())
}

fn parse_numbers(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|' | '\n' | '\r'))
        .filter_map(|token| token.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(4096)
        .collect()
}

fn surface_degrees(container: &Element) -> Vec<f64> {
    if let Some(raw) = container.get_attribute("data-degrees") {
        let parsed = parse_numbers(&raw);
        if !parsed.is_empty() {
            return parsed;
        }
    }
    let text = container
        .query_selector(".vibe-editor, .vibe-editor-textarea, .doc-editor, .sheet-grid")
        .ok()
        .flatten()
        .and_then(|editor| editor.text_content())
        .or_else(|| container.text_content())
        .unwrap_or_default();
    let parsed = parse_numbers(&text);
    if parsed.is_empty() {
        vec![0.1, 0.5, 0.9]
    } else {
        parsed
    }
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

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

fn local_ramp_up(x: f64, a: f64, b: f64) -> f64 {
    if b <= a {
        return if x >= a { 1.0 } else { 0.0 };
    }
    clamp01((x - a) / (b - a))
}

fn local_ramp_down(x: f64, a: f64, b: f64) -> f64 {
    if b <= a {
        return if x <= a { 1.0 } else { 0.0 };
    }
    clamp01((b - x) / (b - a))
}

fn local_triangular(x: f64, a: f64, m: f64, b: f64) -> f64 {
    if x <= a || x >= b {
        0.0
    } else if (x - m).abs() < f64::EPSILON {
        1.0
    } else if x < m {
        clamp01((x - a) / (m - a))
    } else {
        clamp01((b - x) / (b - m))
    }
}

fn local_trapezoidal(x: f64, a: f64, b: f64, c: f64, d: f64) -> f64 {
    if x <= a || x >= d {
        0.0
    } else if x < b {
        clamp01((x - a) / (b - a))
    } else if x <= c {
        1.0
    } else {
        clamp01((d - x) / (d - c))
    }
}

fn local_approximately(x: f64, target: f64, tol: f64) -> f64 {
    if tol <= 0.0 {
        return if (x - target).abs() < f64::EPSILON {
            1.0
        } else {
            0.0
        };
    }
    local_triangular(x, target - tol, target, target + tol)
}

fn local_much_greater_than(x: f64, reference: f64, spread: f64) -> f64 {
    local_ramp_up(x, reference, reference + spread.max(0.0))
}

fn local_much_less_than(x: f64, reference: f64, spread: f64) -> f64 {
    local_ramp_down(x, reference - spread.max(0.0), reference)
}

fn local_and(norm: &str, a: f64, b: f64) -> f64 {
    match norm {
        "product" => a * b,
        "lukasiewicz" => (a + b - 1.0).max(0.0),
        _ => a.min(b),
    }
}

fn local_or(norm: &str, a: f64, b: f64) -> f64 {
    match norm {
        "product" => a + b - a * b,
        "lukasiewicz" => (a + b).min(1.0),
        _ => a.max(b),
    }
}

fn local_not(_norm: &str, a: f64) -> f64 {
    1.0 - a
}

fn local_threshold(degrees: &[f64], alpha: f64) -> Vec<f64> {
    degrees
        .iter()
        .copied()
        .filter(|d| *d >= alpha)
        .collect()
}

fn local_top_k(degrees: &[f64], k: usize) -> Vec<f64> {
    let mut sorted = degrees.to_vec();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    sorted.truncate(k);
    sorted
}

fn format_degrees(degrees: &[f64]) -> String {
    degrees
        .iter()
        .map(|d| format!("{d:.4}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `FuzzyQuery.triangular` — `{ x, a, m, b }`.
pub(super) fn run_fuzzy_triangular(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-x / data-a / data-m / data-b before triangular membership.",
    ) else {
        return;
    };
    let x = surface_f64(&container, "data-x", 25.0);
    let a = surface_f64(&container, "data-a", 20.0);
    let m = surface_f64(&container, "data-m", 30.0);
    let b = surface_f64(&container, "data-b", 40.0);
    let value = local_triangular(x, a, m, b);
    invoke_dual(
        document,
        label,
        "FuzzyQuery.triangular",
        format!(
            "Local triangular sketch: μ({x}; {a},{m},{b}) → {value:.4}. Connect QualiaDB for a live triangular."
        ),
        json!({ "x": x, "a": a, "m": m, "b": b }),
    );
}

/// `FuzzyQuery.trapezoidal` — `{ x, a, b, c, d }`.
pub(super) fn run_fuzzy_trapezoidal(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-x / data-a..data-d before trapezoidal membership.",
    ) else {
        return;
    };
    let x = surface_f64(&container, "data-x", 5.0);
    let a = surface_f64(&container, "data-a", 1.0);
    let b = surface_f64(&container, "data-b", 4.0);
    let c = surface_f64(&container, "data-c", 6.0);
    let d = surface_f64(&container, "data-d", 9.0);
    let value = local_trapezoidal(x, a, b, c, d);
    invoke_dual(
        document,
        label,
        "FuzzyQuery.trapezoidal",
        format!(
            "Local trapezoidal sketch: μ({x}; {a},{b},{c},{d}) → {value:.4}. Connect QualiaDB for a live trapezoidal."
        ),
        json!({ "x": x, "a": a, "b": b, "c": c, "d": d }),
    );
}

/// `FuzzyQuery.approximately` — `{ x, target, tol }`.
pub(super) fn run_fuzzy_approximately(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-x / data-target / data-tol before approximately.",
    ) else {
        return;
    };
    let x = surface_f64(&container, "data-x", 32.5);
    let target = surface_f64(&container, "data-target", 30.0);
    let tol = surface_f64(&container, "data-tol", 5.0);
    let value = local_approximately(x, target, tol);
    invoke_dual(
        document,
        label,
        "FuzzyQuery.approximately",
        format!(
            "Local approximately sketch: ≈{target}±{tol} at {x} → {value:.4}. Connect QualiaDB for a live approximately."
        ),
        json!({ "x": x, "target": target, "tol": tol }),
    );
}

/// `FuzzyQuery.ramp_up` — `{ x, a, b }`.
pub(super) fn run_fuzzy_ramp_up(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-x / data-a / data-b before ramp_up.",
    ) else {
        return;
    };
    let x = surface_f64(&container, "data-x", 2.0);
    let a = surface_f64(&container, "data-a", 1.0);
    let b = surface_f64(&container, "data-b", 3.0);
    let value = local_ramp_up(x, a, b);
    invoke_dual(
        document,
        label,
        "FuzzyQuery.ramp_up",
        format!(
            "Local ramp_up sketch: μ({x}; {a}→{b}) → {value:.4}. Connect QualiaDB for a live ramp_up."
        ),
        json!({ "x": x, "a": a, "b": b }),
    );
}

/// `FuzzyQuery.ramp_down` — `{ x, a, b }`.
pub(super) fn run_fuzzy_ramp_down(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-x / data-a / data-b before ramp_down.",
    ) else {
        return;
    };
    let x = surface_f64(&container, "data-x", 2.0);
    let a = surface_f64(&container, "data-a", 1.0);
    let b = surface_f64(&container, "data-b", 3.0);
    let value = local_ramp_down(x, a, b);
    invoke_dual(
        document,
        label,
        "FuzzyQuery.ramp_down",
        format!(
            "Local ramp_down sketch: μ({x}; {a}→{b}) → {value:.4}. Connect QualiaDB for a live ramp_down."
        ),
        json!({ "x": x, "a": a, "b": b }),
    );
}

/// `FuzzyQuery.much_greater_than` — `{ x, reference, spread }`.
pub(super) fn run_fuzzy_much_greater_than(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-x / data-reference / data-spread before much_greater_than.",
    ) else {
        return;
    };
    let x = surface_f64(&container, "data-x", 100.0);
    let reference = surface_f64(&container, "data-reference", 50.0);
    let spread = surface_f64(&container, "data-spread", 50.0);
    let value = local_much_greater_than(x, reference, spread);
    invoke_dual(
        document,
        label,
        "FuzzyQuery.much_greater_than",
        format!(
            "Local much_greater_than sketch: {x} ≫ {reference}±{spread} → {value:.4}. Connect QualiaDB for a live much_greater_than."
        ),
        json!({ "x": x, "reference": reference, "spread": spread }),
    );
}

/// `FuzzyQuery.much_less_than` — `{ x, reference, spread }`.
pub(super) fn run_fuzzy_much_less_than(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-x / data-reference / data-spread before much_less_than.",
    ) else {
        return;
    };
    let x = surface_f64(&container, "data-x", 0.0);
    let reference = surface_f64(&container, "data-reference", 50.0);
    let spread = surface_f64(&container, "data-spread", 50.0);
    let value = local_much_less_than(x, reference, spread);
    invoke_dual(
        document,
        label,
        "FuzzyQuery.much_less_than",
        format!(
            "Local much_less_than sketch: {x} ≪ {reference}±{spread} → {value:.4}. Connect QualiaDB for a live much_less_than."
        ),
        json!({ "x": x, "reference": reference, "spread": spread }),
    );
}

/// `FuzzyQuery.threshold` — `{ degrees, alpha }`.
pub(super) fn run_fuzzy_threshold(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-degrees (or listed degrees) and data-alpha before threshold.",
    ) else {
        return;
    };
    let degrees = surface_degrees(&container);
    let alpha = surface_f64(&container, "data-alpha", 0.5);
    let kept = local_threshold(&degrees, alpha);
    invoke_dual(
        document,
        label,
        "FuzzyQuery.threshold",
        format!(
            "Local threshold sketch: α={alpha} keeps [{}]. Connect QualiaDB for a live threshold.",
            format_degrees(&kept)
        ),
        json!({ "degrees": degrees, "alpha": alpha }),
    );
}

/// `FuzzyQuery.top_k` — `{ degrees, k }`.
pub(super) fn run_fuzzy_top_k(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-degrees and data-k before top_k.",
    ) else {
        return;
    };
    let degrees = surface_degrees(&container);
    let k = surface_u64(&container, "data-k", 2) as usize;
    let kept = local_top_k(&degrees, k);
    invoke_dual(
        document,
        label,
        "FuzzyQuery.top_k",
        format!(
            "Local top_k sketch: k={k} → [{}]. Connect QualiaDB for a live top_k.",
            format_degrees(&kept)
        ),
        json!({ "degrees": degrees, "k": k as u64 }),
    );
}

/// `FuzzyQuery.negate` — `{ degrees, norm? }`.
pub(super) fn run_fuzzy_negate(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-degrees before negate.",
    ) else {
        return;
    };
    let degrees = surface_degrees(&container);
    let norm = surface_norm(&container);
    let out: Vec<f64> = degrees.iter().map(|d| local_not(&norm, *d)).collect();
    invoke_dual(
        document,
        label,
        "FuzzyQuery.negate",
        format!(
            "Local negate sketch ({norm}): [{}] → [{}]. Connect QualiaDB for a live negate.",
            format_degrees(&degrees),
            format_degrees(&out)
        ),
        json!({ "degrees": degrees, "norm": norm }),
    );
}

/// `FuzzyQuery.and` — `{ a, b, norm? }`.
pub(super) fn run_fuzzy_and(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-a and data-b before fuzzy and.",
    ) else {
        return;
    };
    let a = surface_f64(&container, "data-a", 0.3);
    let b = surface_f64(&container, "data-b", 0.7);
    let norm = surface_norm(&container);
    let value = local_and(&norm, a, b);
    invoke_dual(
        document,
        label,
        "FuzzyQuery.and",
        format!(
            "Local and sketch ({norm}): min-family({a}, {b}) → {value:.4}. Connect QualiaDB for a live and."
        ),
        json!({ "a": a, "b": b, "norm": norm }),
    );
}

/// `FuzzyQuery.or` — `{ a, b, norm? }`.
pub(super) fn run_fuzzy_or(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-a and data-b before fuzzy or.",
    ) else {
        return;
    };
    let a = surface_f64(&container, "data-a", 0.3);
    let b = surface_f64(&container, "data-b", 0.7);
    let norm = surface_norm(&container);
    let value = local_or(&norm, a, b);
    invoke_dual(
        document,
        label,
        "FuzzyQuery.or",
        format!(
            "Local or sketch ({norm}): max-family({a}, {b}) → {value:.4}. Connect QualiaDB for a live or."
        ),
        json!({ "a": a, "b": b, "norm": norm }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_membership_matches_known() {
        assert!((local_triangular(25.0, 20.0, 30.0, 40.0) - 0.5).abs() < 1e-9);
        assert!((local_trapezoidal(5.0, 1.0, 4.0, 6.0, 9.0) - 1.0).abs() < 1e-9);
        assert!((local_approximately(32.5, 30.0, 5.0) - 0.5).abs() < 1e-9);
        assert!((local_ramp_up(2.0, 1.0, 3.0) - 0.5).abs() < 1e-9);
        assert!((local_ramp_down(2.0, 1.0, 3.0) - 0.5).abs() < 1e-9);
        assert!((local_much_greater_than(100.0, 50.0, 50.0) - 1.0).abs() < 1e-9);
        assert!((local_much_less_than(0.0, 50.0, 50.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn local_degree_algebra_matches_known() {
        assert!((local_and("godel", 0.3, 0.7) - 0.3).abs() < 1e-9);
        assert!((local_or("godel", 0.3, 0.7) - 0.7).abs() < 1e-9);
        assert!((local_and("product", 0.5, 0.5) - 0.25).abs() < 1e-9);
        assert!((local_not("godel", 0.2) - 0.8).abs() < 1e-9);
        assert_eq!(local_threshold(&[0.1, 0.5, 0.9], 0.5), vec![0.5, 0.9]);
        assert_eq!(local_top_k(&[0.1, 0.9, 0.5], 2), vec![0.9, 0.5]);
    }
}
