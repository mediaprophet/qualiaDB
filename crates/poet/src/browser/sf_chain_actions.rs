//! Dual-path Tool Chest actions for curated `SpecialFunctions.*` ALL_BOUND ids (wave 15).
//!
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

fn usize_attr(el: Option<&Element>, name: &str) -> Option<usize> {
    numeric_attr(el, name).and_then(|v| {
        if v.is_finite() && v >= 0.0 && v == v.floor() {
            Some(v as usize)
        } else {
            None
        }
    })
}

fn i32_attr(el: Option<&Element>, name: &str) -> Option<i32> {
    numeric_attr(el, name).and_then(|v| {
        if v.is_finite() && v == v.floor() && v >= i32::MIN as f64 && v <= i32::MAX as f64 {
            Some(v as i32)
        } else {
            None
        }
    })
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

// ── Local sketches (offline; Host path is authoritative) ─────────────

fn sketch_airy_ai(x: f64) -> f64 {
    // Ai(0)≈0.355; linear-ish near origin for honesty as a sketch.
    const ALPHA: f64 = 0.355_028_053_887_817_24;
    const BETA: f64 = 0.258_819_403_792_806_8;
    if x.abs() < 1e-12 {
        return ALPHA;
    }
    // Truncated Maclaurin: Ai ≈ α − βx for |x|≲1
    ALPHA - BETA * x + 0.5 * ALPHA * x * x * x / 3.0
}

fn sketch_airy_bi(x: f64) -> f64 {
    const ALPHA: f64 = 0.355_028_053_887_817_24;
    const BETA: f64 = 0.258_819_403_792_806_8;
    let s3 = 3.0_f64.sqrt();
    if x.abs() < 1e-12 {
        return s3 * ALPHA;
    }
    s3 * (ALPHA + BETA * x)
}

fn sketch_zeta(s: f64) -> Option<f64> {
    if s <= 1.0 {
        return None;
    }
    let mut sum = 0.0;
    for n in 1..=64u32 {
        sum += (n as f64).powf(-s);
    }
    Some(sum)
}

fn sketch_legendre(n: u32, x: f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    let (mut p0, mut p1) = (1.0, x);
    for k in 1..n.min(64) {
        let kf = k as f64;
        let p2 = ((2.0 * kf + 1.0) * x * p1 - kf * p0) / (kf + 1.0);
        p0 = p1;
        p1 = p2;
    }
    p1
}

fn sketch_chebyshev_t(n: u32, x: f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    let (mut t0, mut t1) = (1.0, x);
    for _ in 1..n.min(64) {
        let t2 = 2.0 * x * t1 - t0;
        t0 = t1;
        t1 = t2;
    }
    t1
}

fn sketch_chebyshev_u(n: u32, x: f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    let (mut u0, mut u1) = (1.0, 2.0 * x);
    for _ in 1..n.min(64) {
        let u2 = 2.0 * x * u1 - u0;
        u0 = u1;
        u1 = u2;
    }
    u1
}

fn sketch_hermite(n: u32, x: f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    let (mut h0, mut h1) = (1.0, 2.0 * x);
    for k in 1..n.min(64) {
        let h2 = 2.0 * x * h1 - 2.0 * k as f64 * h0;
        h0 = h1;
        h1 = h2;
    }
    h1
}

fn sketch_laguerre(n: u32, x: f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    let (mut l0, mut l1) = (1.0, 1.0 - x);
    for k in 1..n.min(64) {
        let kf = k as f64;
        let l2 = ((2.0 * kf + 1.0 - x) * l1 - kf * l0) / (kf + 1.0);
        l0 = l1;
        l1 = l2;
    }
    l1
}

fn sketch_bessel_j0(x: f64) -> f64 {
    // Truncated J_0 series: 1 − (x/2)² + (x/2)⁴/4 − …
    let h2 = (x * 0.5) * (x * 0.5);
    let mut t = 1.0;
    let mut sum = 0.0;
    let mut sign = 1.0;
    for m in 0..24u32 {
        sum += sign * t;
        t *= h2 / (((m + 1) as f64) * ((m + 1) as f64));
        sign = -sign;
        if t.abs() < 1e-18 {
            break;
        }
    }
    sum
}

fn sketch_bessel_j(n: i32, x: f64) -> f64 {
    let m = n.unsigned_abs().min(8);
    if m == 0 {
        return sketch_bessel_j0(x);
    }
    // Crude: J_1 ≈ x/2 near origin; higher via recurrence sketch from J_0,J_1.
    let j0 = sketch_bessel_j0(x);
    let j1 = if x.abs() < 1e-12 {
        0.0
    } else {
        x * 0.5 * (1.0 - (x * x) / 8.0)
    };
    if m == 1 {
        let v = j1;
        return if n < 0 { -v } else { v };
    }
    let (mut jm1, mut jn) = (j0, j1);
    for k in 1..m {
        let jp1 = if x.abs() < 1e-12 {
            0.0
        } else {
            (2.0 * k as f64 / x) * jn - jm1
        };
        jm1 = jn;
        jn = jp1;
    }
    if n < 0 && m % 2 == 1 {
        -jn
    } else {
        jn
    }
}

fn sketch_bessel_i(n: i32, x: f64) -> f64 {
    let m = n.unsigned_abs().min(8);
    // I_0 ≈ 1 + (x/2)² + …
    let h2 = (x * 0.5) * (x * 0.5);
    let mut t = 1.0;
    let mut i0 = 0.0;
    for k in 0..24u32 {
        i0 += t;
        t *= h2 / (((k + 1) as f64) * ((k + 1) as f64));
        if t.abs() < 1e-18 {
            break;
        }
    }
    if m == 0 {
        return i0;
    }
    let i1 = if x.abs() < 1e-12 {
        0.0
    } else {
        x * 0.5 * (1.0 + (x * x) / 8.0)
    };
    if m == 1 {
        return i1;
    }
    let (mut im1, mut inn) = (i0, i1);
    for k in 1..m {
        let ip1 = if x.abs() < 1e-12 {
            0.0
        } else {
            (2.0 * k as f64 / x) * inn + im1
        };
        im1 = inn;
        inn = ip1;
    }
    inn
}

fn sketch_bessel_y(n: u32, x: f64) -> Option<f64> {
    if x <= 0.0 {
        return None;
    }
    // Rough Y_0 ≈ (2/π)(ln(x/2)+γ) near moderate x
    let y0 = (2.0 / std::f64::consts::PI)
        * ((x * 0.5).ln() + 0.577_215_664_901_532_9)
        * sketch_bessel_j0(x);
    if n == 0 {
        return Some(y0);
    }
    // Placeholder higher orders — Host is authoritative.
    Some(y0 - (n as f64) * 0.1 / x)
}

fn sketch_bessel_k(n: u32, x: f64) -> Option<f64> {
    if x <= 0.0 {
        return None;
    }
    // K_0 ≈ −(ln(x/2)+γ) near small x
    let k0 = -((x * 0.5).ln() + 0.577_215_664_901_532_9);
    if n == 0 {
        return Some(k0);
    }
    Some(k0 + (n as f64) * 0.5 / x)
}

fn resolve_x(document: &Document, nums: &[f64], default: f64) -> f64 {
    let container = selected_container(document);
    numeric_attr(container.as_ref(), "data-x")
        .or_else(|| nums.first().copied())
        .unwrap_or(default)
}

fn resolve_nx(document: &Document, nums: &[f64], default_n: u32, default_x: f64) -> (u32, f64) {
    let container = selected_container(document);
    let n = usize_attr(container.as_ref(), "data-n")
        .or_else(|| nums.first().map(|v| *v as usize))
        .unwrap_or(default_n as usize)
        .min(64) as u32;
    let x = numeric_attr(container.as_ref(), "data-x")
        .or_else(|| {
            if nums.len() >= 2 {
                Some(nums[1])
            } else if nums.len() == 1 && usize_attr(container.as_ref(), "data-n").is_some() {
                None
            } else if nums.len() == 1 {
                None
            } else {
                None
            }
        })
        .or_else(|| {
            if usize_attr(container.as_ref(), "data-n").is_some() {
                nums.first().copied()
            } else if nums.len() >= 2 {
                Some(nums[1])
            } else {
                None
            }
        })
        .unwrap_or(default_x);
    (n, x)
}

fn resolve_signed_nx(
    document: &Document,
    nums: &[f64],
    default_n: i32,
    default_x: f64,
) -> (i32, f64) {
    let container = selected_container(document);
    let n = i32_attr(container.as_ref(), "data-n")
        .or_else(|| nums.first().map(|v| *v as i32))
        .unwrap_or(default_n)
        .clamp(-32, 32);
    let x = numeric_attr(container.as_ref(), "data-x")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(default_x);
    (n, x)
}

/// `SpecialFunctions.airy_ai` — surface `x` or attr `data-x`.
pub(super) fn run_airy_ai(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let x = resolve_x(document, &nums, 0.0);
    if !x.is_finite() {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x (attr data-x or one number).",
            "error",
        );
        return;
    }
    let sketch = sketch_airy_ai(x);
    invoke_dual(
        document,
        label,
        "SpecialFunctions.airy_ai",
        format!("Ai({x}) sketch ≈ {sketch}"),
        json!({ "x": x }),
    );
}

/// `SpecialFunctions.airy_bi` — surface `x` or attr `data-x`.
pub(super) fn run_airy_bi(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let x = resolve_x(document, &nums, 0.0);
    if !x.is_finite() {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x (attr data-x or one number).",
            "error",
        );
        return;
    }
    let sketch = sketch_airy_bi(x);
    invoke_dual(
        document,
        label,
        "SpecialFunctions.airy_bi",
        format!("Bi({x}) sketch ≈ {sketch}"),
        json!({ "x": x }),
    );
}

/// `SpecialFunctions.zeta` — surface `s` or attr `data-s` (requires s>1).
pub(super) fn run_zeta(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let s = numeric_attr(container.as_ref(), "data-s")
        .or_else(|| numeric_attr(container.as_ref(), "data-x"))
        .or_else(|| nums.first().copied())
        .unwrap_or(2.0);
    let Some(sketch) = sketch_zeta(s) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need s>1 (attr data-s or one number).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "SpecialFunctions.zeta",
        format!("ζ({s}) sketch ≈ {sketch}"),
        json!({ "s": s }),
    );
}

/// `SpecialFunctions.legendre` — `n x` or attrs `data-n` / `data-x`.
pub(super) fn run_legendre(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, x) = resolve_nx(document, &nums, 2, 0.5);
    let sketch = sketch_legendre(n, x);
    invoke_dual(
        document,
        label,
        "SpecialFunctions.legendre",
        format!("P_{n}({x}) sketch ≈ {sketch}"),
        json!({ "n": n as u64, "x": x }),
    );
}

/// `SpecialFunctions.chebyshev_t` — `n x` or attrs `data-n` / `data-x`.
pub(super) fn run_chebyshev_t(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, x) = resolve_nx(document, &nums, 2, 0.5);
    let sketch = sketch_chebyshev_t(n, x);
    invoke_dual(
        document,
        label,
        "SpecialFunctions.chebyshev_t",
        format!("T_{n}({x}) sketch ≈ {sketch}"),
        json!({ "n": n as u64, "x": x }),
    );
}

/// `SpecialFunctions.chebyshev_u` — `n x` or attrs `data-n` / `data-x`.
pub(super) fn run_chebyshev_u(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, x) = resolve_nx(document, &nums, 2, 0.5);
    let sketch = sketch_chebyshev_u(n, x);
    invoke_dual(
        document,
        label,
        "SpecialFunctions.chebyshev_u",
        format!("U_{n}({x}) sketch ≈ {sketch}"),
        json!({ "n": n as u64, "x": x }),
    );
}

/// `SpecialFunctions.hermite` — `n x` or attrs `data-n` / `data-x`.
pub(super) fn run_hermite(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, x) = resolve_nx(document, &nums, 2, 0.5);
    let sketch = sketch_hermite(n, x);
    invoke_dual(
        document,
        label,
        "SpecialFunctions.hermite",
        format!("H_{n}({x}) sketch ≈ {sketch}"),
        json!({ "n": n as u64, "x": x }),
    );
}

/// `SpecialFunctions.laguerre` — `n x` or attrs `data-n` / `data-x`.
pub(super) fn run_laguerre(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, x) = resolve_nx(document, &nums, 2, 0.5);
    let sketch = sketch_laguerre(n, x);
    invoke_dual(
        document,
        label,
        "SpecialFunctions.laguerre",
        format!("L_{n}({x}) sketch ≈ {sketch}"),
        json!({ "n": n as u64, "x": x }),
    );
}

/// `SpecialFunctions.bessel_j` — `n x` or attrs `data-n` / `data-x`.
pub(super) fn run_bessel_j(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, x) = resolve_signed_nx(document, &nums, 0, 1.0);
    let sketch = sketch_bessel_j(n, x);
    invoke_dual(
        document,
        label,
        "SpecialFunctions.bessel_j",
        format!("J_{n}({x}) sketch ≈ {sketch}"),
        json!({ "n": n as i64, "x": x }),
    );
}

/// `SpecialFunctions.bessel_i` — `n x` or attrs `data-n` / `data-x`.
pub(super) fn run_bessel_i(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, x) = resolve_signed_nx(document, &nums, 0, 1.0);
    let sketch = sketch_bessel_i(n, x);
    invoke_dual(
        document,
        label,
        "SpecialFunctions.bessel_i",
        format!("I_{n}({x}) sketch ≈ {sketch}"),
        json!({ "n": n as i64, "x": x }),
    );
}

/// `SpecialFunctions.bessel_y` — `n x` with x>0.
pub(super) fn run_bessel_y(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, x) = resolve_nx(document, &nums, 0, 1.0);
    let Some(sketch) = sketch_bessel_y(n, x) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need n≥0 and x>0 (attrs data-n/data-x or two numbers).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "SpecialFunctions.bessel_y",
        format!("Y_{n}({x}) sketch ≈ {sketch}"),
        json!({ "n": n as u64, "x": x }),
    );
}

/// `SpecialFunctions.bessel_k` — `n x` with x>0.
pub(super) fn run_bessel_k(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (n, x) = resolve_nx(document, &nums, 0, 1.0);
    let Some(sketch) = sketch_bessel_k(n, x) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need n≥0 and x>0 (attrs data-n/data-x or two numbers).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "SpecialFunctions.bessel_k",
        format!("K_{n}({x}) sketch ≈ {sketch}"),
        json!({ "n": n as u64, "x": x }),
    );
}
