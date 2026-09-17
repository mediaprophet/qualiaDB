//! Dual-path Tool Chest actions for curated Host-bound `Interpolation.*` ids (wave 24).

use serde_json::json;
use web_sys::{Document, Element};

const DEFAULT_XS: [f64; 2] = [0.0, 1.0];
const DEFAULT_YS: [f64; 2] = [0.0, 1.0];
const DEFAULT_X: f64 = 0.5;
const DEFAULT_DEGREE: u64 = 1;

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

fn local_xy(nums: &[f64]) -> (Vec<f64>, Vec<f64>, f64) {
    if nums.len() >= 5 {
        let mid = nums.len() / 2;
        let xs = nums[..mid].to_vec();
        let ys = nums[mid..nums.len() - 1].to_vec();
        let x = *nums.last().unwrap_or(&DEFAULT_X);
        if xs.len() == ys.len() && xs.len() >= 2 {
            return (xs, ys, x);
        }
    }
    (DEFAULT_XS.to_vec(), DEFAULT_YS.to_vec(), DEFAULT_X)
}

fn local_linear(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    if xs.len() < 2 || ys.len() < 2 {
        return 0.0;
    }
    let x0 = xs[0];
    let x1 = xs[1];
    let y0 = ys[0];
    let y1 = ys[1];
    if (x1 - x0).abs() < 1e-15 {
        return y0;
    }
    y0 + (y1 - y0) * (x - x0) / (x1 - x0)
}

/// `Interpolation.linear_interp` — `{ xs, ys, x }`.
pub(super) fn run_linear(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (xs, ys, x) = local_xy(&nums);
    let sketch = local_linear(&xs, &ys, x);
    invoke_dual(
        document,
        label,
        "Interpolation.linear_interp",
        format!("interp linear sketch x={x} y={sketch}"),
        json!({ "xs": xs, "ys": ys, "x": x }),
    );
}

/// `Interpolation.lagrange_eval` — `{ xs, ys, x }`.
pub(super) fn run_lagrange(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (xs, ys, x) = local_xy(&nums);
    invoke_dual(
        document,
        label,
        "Interpolation.lagrange_eval",
        format!("interp lagrange sketch x={x} n={}", xs.len()),
        json!({ "xs": xs, "ys": ys, "x": x }),
    );
}

/// `Interpolation.newton_coefficients` — `{ xs, ys }`.
pub(super) fn run_newton_coef(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (xs, ys, _) = local_xy(&nums);
    invoke_dual(
        document,
        label,
        "Interpolation.newton_coefficients",
        format!("interp newton_coef sketch n={}", xs.len()),
        json!({ "xs": xs, "ys": ys }),
    );
}

/// `Interpolation.newton_eval` — `{ xs, coef, x }`.
pub(super) fn run_newton_eval(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (xs, ys, x) = local_xy(&nums);
    invoke_dual(
        document,
        label,
        "Interpolation.newton_eval",
        format!("interp newton_eval sketch x={x} n={}", xs.len()),
        json!({ "xs": xs, "coef": ys, "x": x }),
    );
}

/// `Interpolation.poly_fit` — `{ xs, ys, degree }`.
pub(super) fn run_poly_fit(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (xs, ys, _) = local_xy(&nums);
    let degree = numeric_attr(container.as_ref(), "data-degree")
        .map(|v| v.max(0.0) as u64)
        .unwrap_or(DEFAULT_DEGREE);
    invoke_dual(
        document,
        label,
        "Interpolation.poly_fit",
        format!("interp poly_fit sketch n={} degree={degree}", xs.len()),
        json!({ "xs": xs, "ys": ys, "degree": degree }),
    );
}

/// `Interpolation.poly_eval` — `{ coef, x }`.
pub(super) fn run_poly_eval(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (xs, ys, x) = local_xy(&nums);
    let _ = xs;
    invoke_dual(
        document,
        label,
        "Interpolation.poly_eval",
        format!("interp poly_eval sketch x={x} coef_n={}", ys.len()),
        json!({ "coef": ys, "x": x }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_interp_wave24_defaults() {
        let (xs, ys, x) = local_xy(&[]);
        assert_eq!(xs, vec![0.0, 1.0]);
        assert_eq!(ys, vec![0.0, 1.0]);
        assert!((x - 0.5).abs() < 1e-12);
        assert!((local_linear(&xs, &ys, 0.5) - 0.5).abs() < 1e-12);
        assert!((local_linear(&xs, &ys, 0.0) - 0.0).abs() < 1e-12);
    }
}
