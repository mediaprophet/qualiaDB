//! Dual-path Tool Chest actions for curated `PolynomialAlgebra.*` ALL_BOUND ids.
//!
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.
//! Keeps sheet polynomial growth out of `chain_actions.rs` / `stats_chain_actions.rs`.

use serde_json::json;
use web_sys::{Document, Element};

const EPS: f64 = 1e-9;

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
        .take(4096)
        .collect()
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
}

fn usize_attr(el: Option<&Element>, name: &str) -> Option<usize> {
    numeric_attr(el, name).and_then(|v| {
        if v.is_finite() && v >= 0.0 && v == v.floor() {
            Some(v as usize)
        } else {
            None
        }
    })
}

fn trim_poly(mut coeffs: Vec<f64>) -> Vec<f64> {
    while let Some(&c) = coeffs.last() {
        if c.abs() <= EPS {
            coeffs.pop();
        } else {
            break;
        }
    }
    coeffs
}

fn default_poly() -> Vec<f64> {
    vec![1.0, 2.0, 3.0] // 1 + 2x + 3x²
}

fn resolve_one_poly(nums: &[f64], container: Option<&Element>) -> Vec<f64> {
    if let Some(len) = usize_attr(container, "data-a-len").or_else(|| usize_attr(container, "data-len"))
    {
        if len > 0 && len <= nums.len() && len <= 256 {
            return trim_poly(nums[..len].to_vec());
        }
    }
    if nums.is_empty() {
        return default_poly();
    }
    trim_poly(nums.iter().copied().take(256).collect())
}

/// Split sheet numbers into two coefficient lists (a then b).
fn resolve_two_polys(nums: &[f64], container: Option<&Element>) -> (Vec<f64>, Vec<f64>) {
    if let Some(a_len) = usize_attr(container, "data-a-len") {
        if a_len > 0 && a_len < nums.len() && a_len <= 256 {
            let a = trim_poly(nums[..a_len].to_vec());
            let b = trim_poly(nums[a_len..].iter().copied().take(256).collect());
            if !b.is_empty() || a_len < nums.len() {
                return (a, b);
            }
        }
    }
    if nums.len() >= 2 {
        let mid = nums.len() / 2;
        let a = trim_poly(nums[..mid].to_vec());
        let b = trim_poly(nums[mid..].to_vec());
        if !a.is_empty() || !b.is_empty() {
            return (a, b);
        }
    }
    (vec![1.0, 1.0], vec![1.0, -1.0]) // (1+x), (1−x)
}

fn local_eval(a: &[f64], x: f64) -> f64 {
    let mut acc = 0.0;
    for &c in a.iter().rev() {
        acc = acc * x + c;
    }
    acc
}

fn local_add(a: &[f64], b: &[f64]) -> Vec<f64> {
    let n = a.len().max(b.len());
    let mut out = vec![0.0; n];
    for (i, c) in a.iter().enumerate() {
        out[i] += c;
    }
    for (i, c) in b.iter().enumerate() {
        out[i] += c;
    }
    trim_poly(out)
}

fn local_sub(a: &[f64], b: &[f64]) -> Vec<f64> {
    let n = a.len().max(b.len());
    let mut out = vec![0.0; n];
    for (i, c) in a.iter().enumerate() {
        out[i] += c;
    }
    for (i, c) in b.iter().enumerate() {
        out[i] -= c;
    }
    trim_poly(out)
}

fn local_mul(a: &[f64], b: &[f64]) -> Vec<f64> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0.0; a.len() + b.len() - 1];
    for (i, &ai) in a.iter().enumerate() {
        for (j, &bj) in b.iter().enumerate() {
            out[i + j] += ai * bj;
        }
    }
    trim_poly(out)
}

fn local_scale(a: &[f64], s: f64) -> Vec<f64> {
    trim_poly(a.iter().map(|c| c * s).collect())
}

fn local_degree(a: &[f64]) -> Option<usize> {
    if a.is_empty() {
        None
    } else {
        Some(a.len() - 1)
    }
}

fn local_leading(a: &[f64]) -> f64 {
    *a.last().unwrap_or(&0.0)
}

fn local_derivative(a: &[f64]) -> Vec<f64> {
    if a.len() <= 1 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(a.len() - 1);
    for (i, &c) in a.iter().enumerate().skip(1) {
        out.push(c * i as f64);
    }
    trim_poly(out)
}

fn local_monic(a: &[f64]) -> Vec<f64> {
    if a.is_empty() {
        return Vec::new();
    }
    let lead = local_leading(a);
    if lead.abs() <= EPS {
        return Vec::new();
    }
    local_scale(a, 1.0 / lead)
}

fn local_div_rem(a: &[f64], b: &[f64]) -> Option<(Vec<f64>, Vec<f64>)> {
    if b.is_empty() {
        return None;
    }
    let div_deg = b.len() - 1;
    if a.is_empty() || a.len() - 1 < div_deg {
        return Some((Vec::new(), a.to_vec()));
    }
    let mut rem = a.to_vec();
    let div_lead = local_leading(b);
    let quot_len = a.len() - b.len() + 1;
    let mut quot = vec![0.0; quot_len];
    for i in (0..quot_len).rev() {
        let rem_idx = i + div_deg;
        let factor = rem[rem_idx] / div_lead;
        quot[i] = factor;
        if factor != 0.0 {
            for (j, &dc) in b.iter().enumerate() {
                rem[i + j] -= factor * dc;
            }
        }
    }
    Some((trim_poly(quot), trim_poly(rem)))
}

fn local_gcd(a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut aa = a.to_vec();
    let mut bb = b.to_vec();
    while !bb.is_empty() {
        let Some((_, r)) = local_div_rem(&aa, &bb) else {
            break;
        };
        aa = bb;
        bb = r;
    }
    local_monic(&aa)
}

/// Euclidean resultant sketch (matches Host `PolynomialAlgebra.resultant` factors).
fn local_resultant(a: &[f64], b: &[f64]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let mut aa = trim_poly(a.to_vec());
    let mut bb = trim_poly(b.to_vec());
    if aa.is_empty() || bb.is_empty() {
        return 0.0;
    }
    let mut result = 1.0_f64;
    loop {
        let deg_a = aa.len() - 1;
        let deg_b = bb.len() - 1;
        if deg_b == 0 {
            result *= local_leading(&bb).powi(deg_a as i32);
            return result;
        }
        let Some((_, r)) = local_div_rem(&aa, &bb) else {
            return 0.0;
        };
        if (deg_a % 2 == 1) && (deg_b % 2 == 1) {
            result = -result;
        }
        if r.is_empty() {
            return 0.0;
        }
        let deg_r = r.len() - 1;
        result *= local_leading(&bb).powi((deg_a as i32) - (deg_r as i32));
        aa = bb;
        bb = r;
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

/// `PolynomialAlgebra.eval` — Horner at `x` (`data-x` or trailing sheet number).
pub(super) fn run_poly_eval(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (a, x) = if let Some(x) = numeric_attr(container.as_ref(), "data-x") {
        (resolve_one_poly(&nums, container.as_ref()), x)
    } else if nums.len() >= 2 {
        let x = *nums.last().unwrap();
        (trim_poly(nums[..nums.len() - 1].to_vec()), x)
    } else {
        (default_poly(), 2.0)
    };
    let value = local_eval(&a, x);
    invoke_dual(
        document,
        label,
        "PolynomialAlgebra.eval",
        format!("eval sketch p={a:?} at x={x}: {value:.6}"),
        json!({ "a": a, "x": x }),
    );
}

/// `PolynomialAlgebra.add`
pub(super) fn run_poly_add(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (a, b) = resolve_two_polys(&nums, container.as_ref());
    let out = local_add(&a, &b);
    invoke_dual(
        document,
        label,
        "PolynomialAlgebra.add",
        format!("add sketch {a:?} + {b:?} → {out:?}"),
        json!({ "a": a, "b": b }),
    );
}

/// `PolynomialAlgebra.sub`
pub(super) fn run_poly_sub(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (a, b) = resolve_two_polys(&nums, container.as_ref());
    let out = local_sub(&a, &b);
    invoke_dual(
        document,
        label,
        "PolynomialAlgebra.sub",
        format!("sub sketch {a:?} − {b:?} → {out:?}"),
        json!({ "a": a, "b": b }),
    );
}

/// `PolynomialAlgebra.mul`
pub(super) fn run_poly_mul(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (a, b) = resolve_two_polys(&nums, container.as_ref());
    let out = local_mul(&a, &b);
    invoke_dual(
        document,
        label,
        "PolynomialAlgebra.mul",
        format!("mul sketch {a:?} · {b:?} → {out:?}"),
        json!({ "a": a, "b": b }),
    );
}

/// `PolynomialAlgebra.gcd` — monic Euclidean gcd sketch.
pub(super) fn run_poly_gcd(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (a, b) = resolve_two_polys(&nums, container.as_ref());
    let out = local_gcd(&a, &b);
    invoke_dual(
        document,
        label,
        "PolynomialAlgebra.gcd",
        format!("gcd sketch monic({a:?}, {b:?}) → {out:?}"),
        json!({ "a": a, "b": b }),
    );
}

/// `PolynomialAlgebra.degree`
pub(super) fn run_poly_degree(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let a = resolve_one_poly(&nums, container.as_ref());
    let deg = local_degree(&a);
    let msg = match deg {
        Some(d) => format!("degree sketch of {a:?}: {d}"),
        None => format!("degree sketch of zero poly {a:?}: null"),
    };
    invoke_dual(
        document,
        label,
        "PolynomialAlgebra.degree",
        msg,
        json!({ "a": a }),
    );
}

/// `PolynomialAlgebra.leading`
pub(super) fn run_poly_leading(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let a = resolve_one_poly(&nums, container.as_ref());
    let lead = local_leading(&a);
    invoke_dual(
        document,
        label,
        "PolynomialAlgebra.leading",
        format!("leading sketch of {a:?}: {lead:.6}"),
        json!({ "a": a }),
    );
}

/// `PolynomialAlgebra.is_zero`
pub(super) fn run_poly_is_zero(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let a = if nums.is_empty() && container.is_none() {
        Vec::new()
    } else {
        resolve_one_poly(&nums, container.as_ref())
    };
    // Empty sheet → demo non-zero unless explicit empty / data-empty.
    let a = if nums.is_empty()
        && container
            .as_ref()
            .and_then(|e| e.get_attribute("data-empty"))
            .as_deref()
            == Some("1")
    {
        Vec::new()
    } else if nums.is_empty() {
        default_poly()
    } else {
        a
    };
    let zero = a.is_empty();
    invoke_dual(
        document,
        label,
        "PolynomialAlgebra.is_zero",
        format!("is_zero sketch of {a:?}: {zero}"),
        json!({ "a": a }),
    );
}

/// `PolynomialAlgebra.scale` — multiply coeffs by `s` (`data-s` or trailing number).
pub(super) fn run_poly_scale(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (a, s) = if let Some(s) = numeric_attr(container.as_ref(), "data-s") {
        (resolve_one_poly(&nums, container.as_ref()), s)
    } else if nums.len() >= 2 {
        let s = *nums.last().unwrap();
        (trim_poly(nums[..nums.len() - 1].to_vec()), s)
    } else {
        (default_poly(), 2.0)
    };
    let out = local_scale(&a, s);
    invoke_dual(
        document,
        label,
        "PolynomialAlgebra.scale",
        format!("scale sketch {a:?} × {s} → {out:?}"),
        json!({ "a": a, "s": s }),
    );
}

/// `PolynomialAlgebra.zero`
pub(super) fn run_poly_zero(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "PolynomialAlgebra.zero",
        "zero sketch → []".into(),
        json!({}),
    );
}

/// `PolynomialAlgebra.constant` — constant poly from `data-c` or first sheet number.
pub(super) fn run_poly_constant(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let c = numeric_attr(container.as_ref(), "data-c")
        .or_else(|| nums.first().copied())
        .unwrap_or(5.0);
    let out = if c.abs() <= EPS {
        Vec::new()
    } else {
        vec![c]
    };
    invoke_dual(
        document,
        label,
        "PolynomialAlgebra.constant",
        format!("constant sketch c={c} → {out:?}"),
        json!({ "c": c }),
    );
}

/// `PolynomialAlgebra.derivative`
pub(super) fn run_poly_derivative(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let a = resolve_one_poly(&nums, container.as_ref());
    let out = local_derivative(&a);
    invoke_dual(
        document,
        label,
        "PolynomialAlgebra.derivative",
        format!("derivative sketch of {a:?} → {out:?}"),
        json!({ "a": a }),
    );
}

/// `PolynomialAlgebra.monic` — scale so leading coefficient is 1.
pub(super) fn run_poly_monic(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let a = resolve_one_poly(&nums, container.as_ref());
    let out = local_monic(&a);
    invoke_dual(
        document,
        label,
        "PolynomialAlgebra.monic",
        format!("monic sketch of {a:?} → {out:?}"),
        json!({ "a": a }),
    );
}

/// `PolynomialAlgebra.div_rem` — long division `(q, r)`.
pub(super) fn run_poly_div_rem(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (a, b) = resolve_two_polys(&nums, container.as_ref());
    let Some((q, r)) = local_div_rem(&a, &b) else {
        super::interactions::show_tool_status(
            document,
            label,
            "div_rem needs a non-zero divisor polynomial.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "PolynomialAlgebra.div_rem",
        format!("div_rem sketch {a:?} ÷ {b:?} → q={q:?}, r={r:?}"),
        json!({ "a": a, "b": b }),
    );
}

/// `PolynomialAlgebra.resultant` — Euclidean resultant of two sheet polys.
pub(super) fn run_poly_resultant(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (a, b) = resolve_two_polys(&nums, container.as_ref());
    let value = local_resultant(&a, &b);
    invoke_dual(
        document,
        label,
        "PolynomialAlgebra.resultant",
        format!("resultant sketch res({a:?}, {b:?}) = {value:.6}"),
        json!({ "a": a, "b": b }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_monic_scales_leading() {
        let out = local_monic(&[2.0, 4.0]);
        assert!((out[0] - 0.5).abs() < 1e-12);
        assert!((out[1] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_div_rem_exact_x2_minus_1() {
        // (x²−1) / (x−1) → q = x+1, r = 0
        let (q, r) = local_div_rem(&[-1.0, 0.0, 1.0], &[-1.0, 1.0]).unwrap();
        assert!((q[0] - 1.0).abs() < 1e-9);
        assert!((q[1] - 1.0).abs() < 1e-9);
        assert!(r.is_empty() || r.iter().all(|c| c.abs() < 1e-9));
    }

    #[test]
    fn local_resultant_shared_root_near_zero() {
        // (x−1)(x−2) and (x−2) share root 2
        let a = local_mul(&[-1.0, 1.0], &[-2.0, 1.0]);
        let b = vec![-2.0, 1.0];
        let res = local_resultant(&a, &b);
        assert!(res.abs() < 1e-6, "shared-root resultant must be ~0, got {res}");
    }
}
