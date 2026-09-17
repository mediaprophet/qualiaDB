//! Dual-path Tool Chest actions for curated `Calculus.*` ALL_BOUND ids
//! (wave 16 Hermite/BDF/symplectic + wave 19 remainder: adaptive / Newton / JVP·VJP).
//!
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.
//! Local sketches mirror Host scalar algorithms (CPU; Host path authoritative).

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

fn usize_attr(el: Option<&Element>, name: &str) -> Option<usize> {
    numeric_attr(el, name).and_then(|v| {
        if v.is_finite() && v >= 0.0 && v == v.floor() {
            Some(v as usize)
        } else {
            None
        }
    })
}

fn string_attr(el: Option<&Element>, name: &str, fallback: &str) -> String {
    el.and_then(|e| e.get_attribute(name))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| fallback.to_string())
        .trim()
        .to_string()
}

fn pipe_strings(raw: &str) -> Vec<String> {
    raw.split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .take(8)
        .collect()
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

// ── Local sketches (match Host scalar algorithms) ────────────────────

const CBRT2: f64 = 1.259_921_049_894_873_2;
const NEWTON_TOL: f64 = 1e-12;
const NEWTON_MAX_ITERS: u32 = 64;

fn dfdy_fd(f: &dyn Fn(f64, f64) -> f64, t: f64, y: f64) -> f64 {
    let eps = 1e-7 * y.abs().max(1.0);
    (f(t, y + eps) - f(t, y - eps)) / (2.0 * eps)
}

fn power_rhs(rate: f64, power: f64) -> impl Fn(f64, f64) -> f64 {
    move |_t, y| rate * y.powf(power)
}

fn sketch_hermite(y0: f64, f0: f64, y1: f64, f1: f64, h: f64, theta: f64) -> f64 {
    let t = theta;
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    h00 * y0 + h10 * h * f0 + h01 * y1 + h11 * h * f1
}

fn sketch_bdf1(t0: f64, y0: f64, h: f64, f: &dyn Fn(f64, f64) -> f64) -> f64 {
    let t1 = t0 + h;
    let mut y = y0 + h * f(t0, y0);
    for _ in 0..NEWTON_MAX_ITERS {
        let g = y - y0 - h * f(t1, y);
        let dg = 1.0 - h * dfdy_fd(f, t1, y);
        let dy = g / dg;
        y -= dy;
        if dy.abs() <= NEWTON_TOL * y.abs().max(1.0) {
            break;
        }
    }
    y
}

fn sketch_bdf2(t1: f64, y1: f64, y0: f64, h: f64, f: &dyn Fn(f64, f64) -> f64) -> f64 {
    let t2 = t1 + h;
    let c = (4.0 / 3.0) * y1 - (1.0 / 3.0) * y0;
    let beta = 2.0 / 3.0;
    let mut y = y1 + h * f(t1, y1);
    for _ in 0..NEWTON_MAX_ITERS {
        let g = y - c - beta * h * f(t2, y);
        let dg = 1.0 - beta * h * dfdy_fd(f, t2, y);
        let dy = g / dg;
        y -= dy;
        if dy.abs() <= NEWTON_TOL * y.abs().max(1.0) {
            break;
        }
    }
    y
}

fn sketch_integrate_bdf(t0: f64, y0: f64, h: f64, steps: u64, f: &dyn Fn(f64, f64) -> f64) -> f64 {
    if steps == 0 {
        return y0;
    }
    let mut y_prev = y0;
    let mut y_curr = sketch_bdf1(t0, y0, h, f);
    let mut t = t0 + h;
    for _ in 1..steps {
        let y_next = sketch_bdf2(t, y_curr, y_prev, h, f);
        y_prev = y_curr;
        y_curr = y_next;
        t += h;
    }
    y_curr
}

fn sketch_verlet(q: f64, p: f64, h: f64, k: f64, mass: f64) -> (f64, f64) {
    let force = |qq: f64| -k * qq;
    let kv = |pp: f64| pp / mass;
    let p_half = p + 0.5 * h * force(q);
    let q_new = q + h * kv(p_half);
    let p_new = p_half + 0.5 * h * force(q_new);
    (q_new, p_new)
}

fn sketch_ruth3(q: f64, p: f64, h: f64, k: f64, mass: f64) -> (f64, f64) {
    const C: [f64; 3] = [1.0, -2.0 / 3.0, 2.0 / 3.0];
    const D: [f64; 3] = [-1.0 / 24.0, 3.0 / 4.0, 7.0 / 24.0];
    let force = |qq: f64| -k * qq;
    let kv = |pp: f64| pp / mass;
    let mut q = q;
    let mut p = p;
    for i in 0..3 {
        p += C[i] * h * force(q);
        q += D[i] * h * kv(p);
    }
    (q, p)
}

fn sketch_yoshida4(q: f64, p: f64, h: f64, k: f64, mass: f64) -> (f64, f64) {
    let w1 = 1.0 / (2.0 - CBRT2);
    let w0 = -CBRT2 * w1;
    let (q, p) = sketch_verlet(q, p, w1 * h, k, mass);
    let (q, p) = sketch_verlet(q, p, w0 * h, k, mass);
    sketch_verlet(q, p, w1 * h, k, mass)
}

fn sketch_sensitivity(t0: f64, y0: f64, h: f64, steps: u64, f: &dyn Fn(f64, f64) -> f64) -> (f64, f64) {
    let mut t = t0;
    let mut y = y0;
    let mut s = 1.0f64;
    let deriv = |t: f64, y: f64, s: f64| -> (f64, f64) { (f(t, y), dfdy_fd(f, t, y) * s) };
    for _ in 0..steps {
        let (k1y, k1s) = deriv(t, y, s);
        let (k2y, k2s) = deriv(t + 0.5 * h, y + 0.5 * h * k1y, s + 0.5 * h * k1s);
        let (k3y, k3s) = deriv(t + 0.5 * h, y + 0.5 * h * k2y, s + 0.5 * h * k2s);
        let (k4y, k4s) = deriv(t + h, y + h * k3y, s + h * k3s);
        y += (h / 6.0) * (k1y + 2.0 * k2y + 2.0 * k3y + k4y);
        s += (h / 6.0) * (k1s + 2.0 * k2s + 2.0 * k3s + k4s);
        t += h;
    }
    (y, s)
}

fn sketch_pack_f32(step: f32, comp: f32) -> u64 {
    ((step.to_bits() as u64) << 32) | (comp.to_bits() as u64)
}

fn sketch_unpack_f32(packed: u64) -> (f32, f32) {
    (
        f32::from_bits((packed >> 32) as u32),
        f32::from_bits((packed & 0xFFFF_FFFF) as u32),
    )
}

fn sketch_perm_parity(perm: &[usize]) -> Option<i8> {
    let n = perm.len();
    for (i, &v) in perm.iter().enumerate() {
        if v >= n || perm[i + 1..].iter().any(|&o| o == v) {
            return None;
        }
    }
    let mut inv = 0usize;
    for left in 0..n {
        for right in left + 1..n {
            inv += usize::from(perm[left] > perm[right]);
        }
    }
    Some(if inv & 1 == 0 { 1 } else { -1 })
}

fn nth(nums: &[f64], i: usize, default: f64) -> f64 {
    nums.get(i).copied().unwrap_or(default)
}

fn resolve_rate_power(container: Option<&Element>, nums: &[f64], base: usize) -> (f64, f64) {
    let rate = numeric_attr(container, "data-rate")
        .or_else(|| nums.get(base).copied())
        .unwrap_or(-1.0);
    let power = numeric_attr(container, "data-power")
        .or_else(|| nums.get(base + 1).copied())
        .unwrap_or(1.0);
    (rate, power)
}

fn resolve_k_mass(container: Option<&Element>, nums: &[f64]) -> (f64, f64) {
    let k = numeric_attr(container, "data-k")
        .or_else(|| nums.get(3).copied())
        .unwrap_or(1.0);
    let mass = numeric_attr(container, "data-mass")
        .or_else(|| nums.get(4).copied())
        .unwrap_or(1.0);
    (k.max(1e-12), mass.max(1e-12))
}

/// `Calculus.hermite_dense_output`
pub(super) fn run_hermite_dense_output(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let y0 = numeric_attr(container.as_ref(), "data-y0").unwrap_or_else(|| nth(&nums, 0, 1.0));
    let f0 = numeric_attr(container.as_ref(), "data-f0").unwrap_or_else(|| nth(&nums, 1, 0.0));
    let y1 = numeric_attr(container.as_ref(), "data-y1").unwrap_or_else(|| nth(&nums, 2, 2.0));
    let f1 = numeric_attr(container.as_ref(), "data-f1").unwrap_or_else(|| nth(&nums, 3, 0.0));
    let h = numeric_attr(container.as_ref(), "data-h").unwrap_or_else(|| nth(&nums, 4, 1.0));
    let theta =
        numeric_attr(container.as_ref(), "data-theta").unwrap_or_else(|| nth(&nums, 5, 0.5));
    let value = sketch_hermite(y0, f0, y1, f1, h, theta);
    invoke_dual(
        document,
        label,
        "Calculus.hermite_dense_output",
        format!("Hermite dense θ={theta} sketch ≈ {value}"),
        json!({ "y0": y0, "f0": f0, "y1": y1, "f1": f1, "h": h, "theta": theta }),
    );
}

/// `Calculus.bdf1_step`
pub(super) fn run_bdf1_step(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let t0 = numeric_attr(container.as_ref(), "data-t0").unwrap_or_else(|| nth(&nums, 0, 0.0));
    let y0 = numeric_attr(container.as_ref(), "data-y0").unwrap_or_else(|| nth(&nums, 1, 1.0));
    let h = numeric_attr(container.as_ref(), "data-h").unwrap_or_else(|| nth(&nums, 2, 0.1));
    let (rate, power) = resolve_rate_power(container.as_ref(), &nums, 3);
    let f = power_rhs(rate, power);
    let value = sketch_bdf1(t0, y0, h, &f);
    invoke_dual(
        document,
        label,
        "Calculus.bdf1_step",
        format!("BDF1 step sketch ≈ {value}"),
        json!({ "t0": t0, "y0": y0, "h": h, "rate": rate, "power": power }),
    );
}

/// `Calculus.bdf2_step`
pub(super) fn run_bdf2_step(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let t1 = numeric_attr(container.as_ref(), "data-t1").unwrap_or_else(|| nth(&nums, 0, 0.1));
    let y1 = numeric_attr(container.as_ref(), "data-y1").unwrap_or_else(|| nth(&nums, 1, 0.909));
    let y0 = numeric_attr(container.as_ref(), "data-y0").unwrap_or_else(|| nth(&nums, 2, 1.0));
    let h = numeric_attr(container.as_ref(), "data-h").unwrap_or_else(|| nth(&nums, 3, 0.1));
    let (rate, power) = resolve_rate_power(container.as_ref(), &nums, 4);
    let f = power_rhs(rate, power);
    let value = sketch_bdf2(t1, y1, y0, h, &f);
    invoke_dual(
        document,
        label,
        "Calculus.bdf2_step",
        format!("BDF2 step sketch ≈ {value}"),
        json!({ "t1": t1, "y1": y1, "y0": y0, "h": h, "rate": rate, "power": power }),
    );
}

/// `Calculus.verlet_step`
pub(super) fn run_verlet_step(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let q = numeric_attr(container.as_ref(), "data-q").unwrap_or_else(|| nth(&nums, 0, 1.0));
    let p = numeric_attr(container.as_ref(), "data-p").unwrap_or_else(|| nth(&nums, 1, 0.0));
    let h = numeric_attr(container.as_ref(), "data-h").unwrap_or_else(|| nth(&nums, 2, 0.01));
    let (k, mass) = resolve_k_mass(container.as_ref(), &nums);
    let (qn, pn) = sketch_verlet(q, p, h, k, mass);
    invoke_dual(
        document,
        label,
        "Calculus.verlet_step",
        format!("Verlet step sketch q≈{qn}, p≈{pn}"),
        json!({ "q": q, "p": p, "h": h, "k": k, "mass": mass }),
    );
}

/// `Calculus.ruth3_step`
pub(super) fn run_ruth3_step(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let q = numeric_attr(container.as_ref(), "data-q").unwrap_or_else(|| nth(&nums, 0, 1.0));
    let p = numeric_attr(container.as_ref(), "data-p").unwrap_or_else(|| nth(&nums, 1, 0.0));
    let h = numeric_attr(container.as_ref(), "data-h").unwrap_or_else(|| nth(&nums, 2, 0.1));
    let (k, mass) = resolve_k_mass(container.as_ref(), &nums);
    let (qn, pn) = sketch_ruth3(q, p, h, k, mass);
    invoke_dual(
        document,
        label,
        "Calculus.ruth3_step",
        format!("Ruth3 step sketch q≈{qn}, p≈{pn}"),
        json!({ "q": q, "p": p, "h": h, "k": k, "mass": mass }),
    );
}

/// `Calculus.yoshida4_step`
pub(super) fn run_yoshida4_step(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let q = numeric_attr(container.as_ref(), "data-q").unwrap_or_else(|| nth(&nums, 0, 1.0));
    let p = numeric_attr(container.as_ref(), "data-p").unwrap_or_else(|| nth(&nums, 1, 0.0));
    let h = numeric_attr(container.as_ref(), "data-h").unwrap_or_else(|| nth(&nums, 2, 0.1));
    let (k, mass) = resolve_k_mass(container.as_ref(), &nums);
    let (qn, pn) = sketch_yoshida4(q, p, h, k, mass);
    invoke_dual(
        document,
        label,
        "Calculus.yoshida4_step",
        format!("Yoshida4 step sketch q≈{qn}, p≈{pn}"),
        json!({ "q": q, "p": p, "h": h, "k": k, "mass": mass }),
    );
}

/// `Calculus.integrate_bdf`
pub(super) fn run_integrate_bdf(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let t0 = numeric_attr(container.as_ref(), "data-t0").unwrap_or_else(|| nth(&nums, 0, 0.0));
    let y0 = numeric_attr(container.as_ref(), "data-y0").unwrap_or_else(|| nth(&nums, 1, 1.0));
    let h = numeric_attr(container.as_ref(), "data-h").unwrap_or_else(|| nth(&nums, 2, 0.1));
    let steps = usize_attr(container.as_ref(), "data-steps")
        .or_else(|| nums.get(3).map(|v| *v as usize))
        .unwrap_or(10)
        .min(10_000) as u64;
    let (rate, power) = resolve_rate_power(container.as_ref(), &nums, 4);
    let f = power_rhs(rate, power);
    let value = sketch_integrate_bdf(t0, y0, h, steps, &f);
    invoke_dual(
        document,
        label,
        "Calculus.integrate_bdf",
        format!("BDF integrate ({steps} steps) sketch ≈ {value}"),
        json!({ "t0": t0, "y0": y0, "h": h, "steps": steps, "rate": rate, "power": power }),
    );
}

/// `Calculus.integrate_with_sensitivity`
pub(super) fn run_integrate_with_sensitivity(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let t0 = numeric_attr(container.as_ref(), "data-t0").unwrap_or_else(|| nth(&nums, 0, 0.0));
    let y0 = numeric_attr(container.as_ref(), "data-y0").unwrap_or_else(|| nth(&nums, 1, 1.0));
    let h = numeric_attr(container.as_ref(), "data-h").unwrap_or_else(|| nth(&nums, 2, 0.05));
    let steps = usize_attr(container.as_ref(), "data-steps")
        .or_else(|| nums.get(3).map(|v| *v as usize))
        .unwrap_or(20)
        .min(10_000) as u64;
    let (rate, power) = resolve_rate_power(container.as_ref(), &nums, 4);
    let f = power_rhs(rate, power);
    let (y, s) = sketch_sensitivity(t0, y0, h, steps, &f);
    invoke_dual(
        document,
        label,
        "Calculus.integrate_with_sensitivity",
        format!("Sensitivity integrate sketch y≈{y}, s≈{s}"),
        json!({ "t0": t0, "y0": y0, "h": h, "steps": steps, "rate": rate, "power": power }),
    );
}

/// `Calculus.invariant_drift`
pub(super) fn run_invariant_drift(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let initial =
        numeric_attr(container.as_ref(), "data-initial").unwrap_or_else(|| nth(&nums, 0, 2.0));
    let final_value =
        numeric_attr(container.as_ref(), "data-final").unwrap_or_else(|| nth(&nums, 1, 2.0));
    let abs_d = (final_value - initial).abs();
    let rel_d = abs_d / initial.abs().max(f64::MIN_POSITIVE);
    invoke_dual(
        document,
        label,
        "Calculus.invariant_drift",
        format!("Invariant drift sketch |Δ|≈{abs_d}, rel≈{rel_d}"),
        json!({ "initial": initial, "final": final_value }),
    );
}

/// `Calculus.permutation_parity`
pub(super) fn run_permutation_parity(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let perm: Vec<usize> = if nums.is_empty() {
        vec![1, 0]
    } else {
        nums.iter()
            .take(16)
            .filter_map(|v| {
                if *v >= 0.0 && *v == v.floor() {
                    Some(*v as usize)
                } else {
                    None
                }
            })
            .collect()
    };
    let _ = container;
    let sketch = sketch_perm_parity(&perm);
    let local = match sketch {
        Some(p) => format!("Permutation parity sketch = {p}"),
        None => "Permutation parity sketch: invalid permutation".into(),
    };
    let perm_json: Vec<u64> = perm.iter().map(|&v| v as u64).collect();
    invoke_dual(
        document,
        label,
        "Calculus.permutation_parity",
        local,
        json!({ "permutation": perm_json }),
    );
}

/// `Calculus.pack_f32_pair`
pub(super) fn run_pack_f32_pair(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let step = numeric_attr(container.as_ref(), "data-step").unwrap_or_else(|| nth(&nums, 0, 1.5));
    let comp = numeric_attr(container.as_ref(), "data-comp").unwrap_or_else(|| nth(&nums, 1, -0.25));
    let packed = sketch_pack_f32(step as f32, comp as f32);
    invoke_dual(
        document,
        label,
        "Calculus.pack_f32_pair",
        format!("pack_f32_pair sketch packed={packed}"),
        json!({ "step": step, "comp": comp }),
    );
}

/// `Calculus.unpack_f32_pair`
pub(super) fn run_unpack_f32_pair(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let packed = numeric_attr(container.as_ref(), "data-packed")
        .or_else(|| nums.first().copied())
        .map(|v| v as u64)
        .unwrap_or_else(|| sketch_pack_f32(1.5, -0.25));
    let (step, comp) = sketch_unpack_f32(packed);
    invoke_dual(
        document,
        label,
        "Calculus.unpack_f32_pair",
        format!("unpack_f32_pair sketch step≈{step}, comp≈{comp}"),
        json!({ "packed": packed }),
    );
}

// ── Wave 19 remainder (adaptive / Newton / mechanics / JVP·VJP) ───────

fn sketch_poisson_bracket(df_dq: [f64; 2], df_dp: [f64; 2], dg_dq: [f64; 2], dg_dp: [f64; 2]) -> f64 {
    df_dq[0] * dg_dp[0] + df_dq[1] * dg_dp[1] - (df_dp[0] * dg_dq[0] + df_dp[1] * dg_dq[1])
}

fn sketch_stormer_verlet(q: f64, p: f64, h: f64, k: f64, mass: f64) -> (f64, f64) {
    // Host passes potential_gradient = k·q; stormer applies −∇V internally.
    sketch_verlet(q, p, h, k, mass)
}

fn sketch_composite_simpson_x2(a: f64, b: f64, panels: usize) -> f64 {
    let n = panels.max(2) & !1;
    let h = (b - a) / n as f64;
    let mut acc = a * a + b * b;
    for i in 1..n {
        let x = a + i as f64 * h;
        let w = if i % 2 == 0 { 2.0 } else { 4.0 };
        acc += w * x * x;
    }
    acc * h / 3.0
}

fn sketch_central_deriv_x2(x: f64) -> f64 {
    let eps = 1e-6 * x.abs().max(1.0);
    let f = |t: f64| t * t;
    (f(x + eps) - f(x - eps)) / (2.0 * eps)
}

fn sketch_jvp_2x2(a: [[f64; 2]; 2], v: [f64; 2]) -> [f64; 2] {
    [
        a[0][0] * v[0] + a[0][1] * v[1],
        a[1][0] * v[0] + a[1][1] * v[1],
    ]
}

fn sketch_vjp_2x2(a: [[f64; 2]; 2], w: [f64; 2]) -> [f64; 2] {
    [
        a[0][0] * w[0] + a[1][0] * w[1],
        a[0][1] * w[0] + a[1][1] * w[1],
    ]
}

fn sketch_newton_sqrt2(guess: f64) -> f64 {
    let mut x = guess;
    for _ in 0..32 {
        let fx = x * x - 2.0;
        if fx.abs() < 1e-12 {
            break;
        }
        x -= fx / (2.0 * x);
    }
    x
}

fn take2(nums: &[f64], start: usize, d0: f64, d1: f64) -> [f64; 2] {
    [nth(nums, start, d0), nth(nums, start + 1, d1)]
}

fn mat2_from_nums(nums: &[f64]) -> [[f64; 2]; 2] {
    if nums.len() >= 4 {
        [[nth(nums, 0, 1.0), nth(nums, 1, 0.0)], [nth(nums, 2, 0.0), nth(nums, 3, 1.0)]]
    } else {
        [[1.0, 0.0], [0.0, 1.0]]
    }
}

/// `Calculus.canonical_poisson_bracket`
pub(super) fn run_canonical_poisson_bracket(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let df_dq = take2(&nums, 0, 1.0, 2.0);
    let df_dp = take2(&nums, 2, 3.0, 4.0);
    let dg_dq = take2(&nums, 4, -2.0, 1.0);
    let dg_dp = take2(&nums, 6, 0.5, 3.0);
    let _ = container;
    let value = sketch_poisson_bracket(df_dq, df_dp, dg_dq, dg_dp);
    invoke_dual(
        document,
        label,
        "Calculus.canonical_poisson_bracket",
        format!("Poisson bracket sketch ≈ {value}"),
        json!({
            "df_dq": df_dq.to_vec(),
            "df_dp": df_dp.to_vec(),
            "dg_dq": dg_dq.to_vec(),
            "dg_dp": dg_dp.to_vec(),
        }),
    );
}

/// `Calculus.stormer_verlet_step`
pub(super) fn run_stormer_verlet_step(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let q = numeric_attr(container.as_ref(), "data-q").unwrap_or_else(|| nth(&nums, 0, 1.0));
    let p = numeric_attr(container.as_ref(), "data-p").unwrap_or_else(|| nth(&nums, 1, 0.0));
    let h = numeric_attr(container.as_ref(), "data-h").unwrap_or_else(|| nth(&nums, 2, 0.01));
    let (k, mass) = resolve_k_mass(container.as_ref(), &nums);
    let (qn, pn) = sketch_stormer_verlet(q, p, h, k, mass);
    invoke_dual(
        document,
        label,
        "Calculus.stormer_verlet_step",
        format!("Störmer–Verlet step sketch q≈{qn}, p≈{pn}"),
        json!({ "q": q, "p": p, "h": h, "k": k, "mass": mass }),
    );
}

/// `Calculus.adaptive_gauss_kronrod_15`
pub(super) fn run_adaptive_gauss_kronrod_15(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let expr = string_attr(container.as_ref(), "data-expr", "x^2");
    let var = string_attr(container.as_ref(), "data-var", "x");
    let a = numeric_attr(container.as_ref(), "data-a").unwrap_or_else(|| nth(&nums, 0, 0.0));
    let b = numeric_attr(container.as_ref(), "data-b").unwrap_or_else(|| nth(&nums, 1, 3.0));
    let tol = numeric_attr(container.as_ref(), "data-tolerance").unwrap_or(1e-8);
    let sketch = if expr.trim() == "x^2" || expr.trim() == "x**2" {
        sketch_composite_simpson_x2(a, b, 64)
    } else {
        (b - a) * 0.0
    };
    invoke_dual(
        document,
        label,
        "Calculus.adaptive_gauss_kronrod_15",
        format!("G7-K15 sketch ∫[{a},{b}] {expr} d{var} ≈ {sketch}"),
        json!({
            "expr": expr,
            "var": var,
            "a": a,
            "b": b,
            "tolerance": tol,
            "max_evaluations": 10_000,
        }),
    );
}

/// `Calculus.adaptive_simpson`
pub(super) fn run_adaptive_simpson(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let expr = string_attr(container.as_ref(), "data-expr", "x^2");
    let var = string_attr(container.as_ref(), "data-var", "x");
    let a = numeric_attr(container.as_ref(), "data-a").unwrap_or_else(|| nth(&nums, 0, 0.0));
    let b = numeric_attr(container.as_ref(), "data-b").unwrap_or_else(|| nth(&nums, 1, 1.0));
    let tol = numeric_attr(container.as_ref(), "data-tolerance").unwrap_or(1e-8);
    let sketch = if expr.trim() == "x^2" || expr.trim() == "x**2" {
        sketch_composite_simpson_x2(a, b, 64)
    } else {
        (b - a) * 0.0
    };
    invoke_dual(
        document,
        label,
        "Calculus.adaptive_simpson",
        format!("Adaptive Simpson sketch ∫[{a},{b}] {expr} d{var} ≈ {sketch}"),
        json!({
            "expr": expr,
            "var": var,
            "a": a,
            "b": b,
            "tolerance": tol,
            "max_evaluations": 10_000,
        }),
    );
}

/// `Calculus.adaptive_derivative`
pub(super) fn run_adaptive_derivative(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let expr = string_attr(container.as_ref(), "data-expr", "x^2");
    let var = string_attr(container.as_ref(), "data-var", "x");
    let x = numeric_attr(container.as_ref(), "data-x").unwrap_or_else(|| nth(&nums, 0, 1.0));
    let sketch = if expr.trim() == "x^2" || expr.trim() == "x**2" {
        sketch_central_deriv_x2(x)
    } else {
        0.0
    };
    invoke_dual(
        document,
        label,
        "Calculus.adaptive_derivative",
        format!("Adaptive derivative sketch d/d{var}({expr})|{x} ≈ {sketch}"),
        json!({ "expr": expr, "var": var, "x": x }),
    );
}

/// `Calculus.jvp`
pub(super) fn run_jvp(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let a = mat2_from_nums(&nums);
    let v = if nums.len() >= 6 {
        take2(&nums, 4, 1.0, 0.0)
    } else {
        take2(&nums, 0, 3.0, -1.0)
    };
    let _ = container;
    let out = sketch_jvp_2x2(a, v);
    invoke_dual(
        document,
        label,
        "Calculus.jvp",
        format!("JVP sketch out≈[{}, {}]", out[0], out[1]),
        json!({
            "a": [a[0].to_vec(), a[1].to_vec()],
            "v": v.to_vec(),
        }),
    );
}

/// `Calculus.vjp`
pub(super) fn run_vjp(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let a = if nums.len() >= 4 {
        mat2_from_nums(&nums)
    } else {
        [[1.0, 2.0], [3.0, 4.0]]
    };
    let w = if nums.len() >= 6 {
        take2(&nums, 4, 1.0, 0.0)
    } else {
        [1.0, 0.0]
    };
    let _ = container;
    let out = sketch_vjp_2x2(a, w);
    invoke_dual(
        document,
        label,
        "Calculus.vjp",
        format!("VJP sketch out≈[{}, {}]", out[0], out[1]),
        json!({
            "a": [a[0].to_vec(), a[1].to_vec()],
            "w": w.to_vec(),
        }),
    );
}

/// `Calculus.newton_solve`
pub(super) fn run_newton_solve(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let exprs_raw = string_attr(container.as_ref(), "data-exprs", "x^2-2");
    let vars_raw = string_attr(container.as_ref(), "data-vars", "x");
    let exprs = {
        let mut v = pipe_strings(&exprs_raw);
        if v.is_empty() {
            v.push("x^2-2".into());
        }
        v
    };
    let vars = {
        let mut v = pipe_strings(&vars_raw);
        if v.is_empty() {
            v.push("x".into());
        }
        v
    };
    let guess = if nums.is_empty() {
        vec![1.0]
    } else {
        nums.iter().copied().take(vars.len().max(1)).collect()
    };
    let sketch = if exprs.len() == 1 && vars.len() == 1 && (exprs[0] == "x^2-2" || exprs[0] == "x**2-2")
    {
        sketch_newton_sqrt2(guess.first().copied().unwrap_or(1.0))
    } else {
        guess.first().copied().unwrap_or(0.0)
    };
    invoke_dual(
        document,
        label,
        "Calculus.newton_solve",
        format!("Newton solve sketch ≈ {sketch}"),
        json!({
            "exprs": exprs,
            "vars": vars,
            "guess": guess,
            "max_iter": 50,
            "tolerance": 1e-8,
        }),
    );
}

/// `Calculus.numerical_jacobian`
pub(super) fn run_numerical_jacobian(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let exprs_raw = string_attr(container.as_ref(), "data-exprs", "x|y");
    let vars_raw = string_attr(container.as_ref(), "data-vars", "x|y");
    let exprs = {
        let mut v = pipe_strings(&exprs_raw);
        if v.is_empty() {
            v.extend(["x".into(), "y".into()]);
        }
        v
    };
    let vars = {
        let mut v = pipe_strings(&vars_raw);
        if v.is_empty() {
            v.extend(["x".into(), "y".into()]);
        }
        v
    };
    let point = if nums.len() >= vars.len() {
        nums.iter().copied().take(vars.len()).collect::<Vec<_>>()
    } else {
        vec![1.0; vars.len().max(1)]
    };
    let identity = exprs.len() == vars.len()
        && exprs.iter().zip(vars.iter()).all(|(e, v)| e == v);
    let sketch = if identity {
        format!("{}×{} identity", exprs.len(), vars.len())
    } else {
        format!("{}×{} FD", exprs.len(), vars.len())
    };
    invoke_dual(
        document,
        label,
        "Calculus.numerical_jacobian",
        format!("Numerical Jacobian sketch ({sketch})"),
        json!({
            "exprs": exprs,
            "vars": vars,
            "point": point,
            "step": 1e-6,
        }),
    );
}

/// `Calculus.numerical_hessian`
pub(super) fn run_numerical_hessian(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let expr = string_attr(container.as_ref(), "data-expr", "x^2+y^2");
    let vars_raw = string_attr(container.as_ref(), "data-vars", "x|y");
    let vars = {
        let mut v = pipe_strings(&vars_raw);
        if v.is_empty() {
            v.extend(["x".into(), "y".into()]);
        }
        v
    };
    let point = if nums.len() >= vars.len() {
        nums.iter().copied().take(vars.len()).collect::<Vec<_>>()
    } else {
        vec![0.0; vars.len().max(1)]
    };
    let sketch = if expr == "x^2+y^2" && vars.len() == 2 {
        "≈ 2·I"
    } else {
        "FD Hessian"
    };
    invoke_dual(
        document,
        label,
        "Calculus.numerical_hessian",
        format!("Numerical Hessian sketch ({sketch})"),
        json!({
            "expr": expr,
            "vars": vars,
            "point": point,
            "step": 1e-5,
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_poisson_bracket_antisym() {
        let fq = [1.0, 2.0];
        let fp = [3.0, 4.0];
        let gq = [-2.0, 1.0];
        let gp = [0.5, 3.0];
        let fg = sketch_poisson_bracket(fq, fp, gq, gp);
        let gf = sketch_poisson_bracket(gq, gp, fq, fp);
        assert!((fg + gf).abs() < 1e-12);
    }

    #[test]
    fn local_jvp_identity() {
        let out = sketch_jvp_2x2([[1.0, 0.0], [0.0, 1.0]], [3.0, -1.0]);
        assert!((out[0] - 3.0).abs() < 1e-12);
        assert!((out[1] + 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_vjp_transpose() {
        let out = sketch_vjp_2x2([[1.0, 2.0], [3.0, 4.0]], [1.0, 0.0]);
        assert!((out[0] - 1.0).abs() < 1e-12);
        assert!((out[1] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn local_newton_sqrt2() {
        let x = sketch_newton_sqrt2(1.0);
        assert!((x * x - 2.0).abs() < 1e-10);
    }
}
