//! Dual-path Tool Chest actions for curated `Chemistry.*` ALL_BOUND ids
//! (wave 14 integrals/angular + wave 18 SCF/element/LDA).
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

fn gto_json(origin: [f64; 3], exponent: f64, l: [u8; 3], coefficient: f64) -> serde_json::Value {
    json!({
        "origin": origin.to_vec(),
        "exponent": exponent,
        "lx": l[0] as u64,
        "ly": l[1] as u64,
        "lz": l[2] as u64,
        "coefficient": coefficient,
    })
}

fn default_s_gto() -> serde_json::Value {
    gto_json([0.0, 0.0, 0.0], 1.0, [0, 0, 0], 1.0)
}

fn default_s_gto_offset() -> serde_json::Value {
    gto_json([0.0, 0.0, 1.0], 1.0, [0, 0, 0], 1.0)
}

/// Offline Boys F_n(t) sketch (Host uses IntegralEngine::boys_function).
fn local_boys(n: u8, t: f64) -> Option<f64> {
    if !t.is_finite() || t < 0.0 || n > 12 {
        return None;
    }
    if t < 1e-8 {
        return Some(1.0 / (2.0 * f64::from(n) + 1.0));
    }
    // Crude downward recurrence sketch from F_0 ≈ ½√(π/t)·erf(√t).
    let sqrt_t = t.sqrt();
    let mut f0 = 0.5 * (std::f64::consts::PI / t).sqrt() * erf_approx(sqrt_t);
    if n == 0 {
        return f0.is_finite().then_some(f0);
    }
    for m in 0..n {
        f0 = ((2.0 * f64::from(m) + 1.0) * f0 - (-t).exp()) / (2.0 * t);
        if !f0.is_finite() {
            return None;
        }
    }
    Some(f0)
}

fn erf_approx(x: f64) -> f64 {
    // Abramowitz–Stegun 7.1.26
    let t = 1.0 / (1.0 + 0.3275911 * x.abs());
    let poly = t
        * (0.254829592
            + t * (-0.284496736
                + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
    let y = 1.0 - poly * (-x * x).exp();
    if x >= 0.0 {
        y
    } else {
        -y
    }
}

fn letter_for(l: u8) -> Option<&'static str> {
    Some(match l {
        0 => "s",
        1 => "p",
        2 => "d",
        3 => "f",
        4 => "g",
        5 => "h",
        _ => return None,
    })
}

fn l_from_letter(c: char) -> Option<u8> {
    match c.to_ascii_lowercase() {
        's' => Some(0),
        'p' => Some(1),
        'd' => Some(2),
        'f' => Some(3),
        'g' => Some(4),
        'h' => Some(5),
        _ => None,
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

/// `Chemistry.boys_function` — surface `n t` or attrs `data-n` / `data-t`.
pub(super) fn run_boys_function(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let n = usize_attr(container.as_ref(), "data-n")
        .or_else(|| nums.first().map(|v| *v as usize))
        .unwrap_or(0)
        .min(12) as u8;
    let t = numeric_attr(container.as_ref(), "data-t")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(0.5);
    let Some(sketch) = local_boys(n, t) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite n∈[0,12] and t≥0 (attrs data-n/data-t or two numbers).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Chemistry.boys_function",
        format!("Boys F_{n}({t}) sketch ≈ {sketch}"),
        json!({ "n": n as u64, "t": t }),
    );
}

fn two_gto_args(document: &Document) -> (serde_json::Value, serde_json::Value, String) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    // Optional: first 8 numbers → a.exp,a.origin×3,b.exp,b.origin×3
    if nums.len() >= 8 {
        let a = gto_json(
            [nums[1], nums[2], nums[3]],
            nums[0],
            [0, 0, 0],
            1.0,
        );
        let b = gto_json(
            [nums[5], nums[6], nums[7]],
            nums[4],
            [0, 0, 0],
            1.0,
        );
        (
            a,
            b,
            format!(
                "two s-GTOs from surface (αa={}, αb={})",
                nums[0], nums[4]
            ),
        )
    } else {
        (
            default_s_gto(),
            default_s_gto_offset(),
            "demo s-GTOs at origin and z=1 (α=1)".into(),
        )
    }
}

pub(super) fn run_overlap_s(document: &Document, label: &str) {
    let (a, b, note) = two_gto_args(document);
    invoke_dual(
        document,
        label,
        "Chemistry.overlap_s",
        format!("overlap_s sketch — {note}"),
        json!({ "a": a, "b": b }),
    );
}

pub(super) fn run_kinetic_s(document: &Document, label: &str) {
    let (a, b, note) = two_gto_args(document);
    invoke_dual(
        document,
        label,
        "Chemistry.kinetic_s",
        format!("kinetic_s sketch — {note}"),
        json!({ "a": a, "b": b }),
    );
}

pub(super) fn run_nuclear_s(document: &Document, label: &str) {
    let (a, b, note) = two_gto_args(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let z = nums.get(8).copied().unwrap_or(1.0);
    let center = if nums.len() >= 12 {
        [nums[9], nums[10], nums[11]]
    } else {
        [0.0, 0.0, 0.0]
    };
    invoke_dual(
        document,
        label,
        "Chemistry.nuclear_s",
        format!("nuclear_s sketch — {note}; Z={z} at {center:?}"),
        json!({ "a": a, "b": b, "center": center.to_vec(), "z": z }),
    );
}

pub(super) fn run_dipole_s(document: &Document, label: &str) {
    let (a, b, note) = two_gto_args(document);
    invoke_dual(
        document,
        label,
        "Chemistry.dipole_s",
        format!("dipole_s sketch — {note}"),
        json!({ "a": a, "b": b }),
    );
}

pub(super) fn run_evaluate_eri(document: &Document, label: &str) {
    let (a, b, note) = two_gto_args(document);
    // (ab|cd) with c=a, d=b demo when surface is short
    invoke_dual(
        document,
        label,
        "Chemistry.evaluate_eri",
        format!("evaluate_eri sketch (ab|ab) — {note}"),
        json!({ "a": a.clone(), "b": b.clone(), "c": a, "d": b }),
    );
}

pub(super) fn run_total_angular_momentum(document: &Document, label: &str) {
    let container = selected_container(document);
    let lx = usize_attr(container.as_ref(), "data-lx").unwrap_or(1) as u8;
    let ly = usize_attr(container.as_ref(), "data-ly").unwrap_or(0) as u8;
    let lz = usize_attr(container.as_ref(), "data-lz").unwrap_or(0) as u8;
    let total = u64::from(lx) + u64::from(ly) + u64::from(lz);
    let gto = gto_json([0.0, 0.0, 0.0], 1.0, [lx, ly, lz], 1.0);
    invoke_dual(
        document,
        label,
        "Chemistry.total_angular_momentum",
        format!("total angular momentum sketch L={total} (lx,ly,lz)=({lx},{ly},{lz})"),
        json!({ "origin": [0.0, 0.0, 0.0], "exponent": 1.0, "lx": lx as u64, "ly": ly as u64, "lz": lz as u64, "coefficient": 1.0, "gto": gto }),
    );
}

pub(super) fn run_letter(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let l = usize_attr(container.as_ref(), "data-l")
        .or_else(|| nums.first().map(|v| *v as usize))
        .unwrap_or(1)
        .min(5) as u8;
    let Some(letter) = letter_for(l) else {
        super::interactions::show_tool_status(document, label, "Need l in 0..=5.", "error");
        return;
    };
    invoke_dual(
        document,
        label,
        "Chemistry.letter",
        format!("spectroscopic letter sketch l={l} → '{letter}'"),
        json!({ "l": l as u64 }),
    );
}

pub(super) fn run_n_cartesian(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let l = usize_attr(container.as_ref(), "data-l")
        .or_else(|| nums.first().map(|v| *v as usize))
        .unwrap_or(1)
        .min(12) as u8;
    let n = (u64::from(l) + 1) * (u64::from(l) + 2) / 2;
    invoke_dual(
        document,
        label,
        "Chemistry.n_cartesian",
        format!("n_cartesian sketch l={l} → {n}"),
        json!({ "l": l as u64 }),
    );
}

pub(super) fn run_n_spherical(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let l = usize_attr(container.as_ref(), "data-l")
        .or_else(|| nums.first().map(|v| *v as usize))
        .unwrap_or(1)
        .min(12) as u8;
    let n = 2 * u64::from(l) + 1;
    invoke_dual(
        document,
        label,
        "Chemistry.n_spherical",
        format!("n_spherical sketch l={l} → {n}"),
        json!({ "l": l as u64 }),
    );
}

pub(super) fn run_from_letter(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let ch = container
        .as_ref()
        .and_then(|e| e.get_attribute("data-letter"))
        .and_then(|s| s.chars().next())
        .or_else(|| {
            source
                .chars()
                .find(|c| c.is_ascii_alphabetic())
        })
        .unwrap_or('p');
    let Some(l) = l_from_letter(ch) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need spectroscopic letter s/p/d/f/g/h (data-letter or surface text).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Chemistry.from_letter",
        format!("from_letter sketch '{ch}' → l={l}"),
        json!({ "letter": ch.to_ascii_lowercase().to_string() }),
    );
}

// ── Wave 18: SCF LA helpers + element/LDA Host caps ───────────────────────

fn nested_square(n: usize, data: &[f64]) -> serde_json::Value {
    let rows: Vec<Vec<f64>> = (0..n)
        .map(|i| data[i * n..(i + 1) * n].to_vec())
        .collect();
    json!(rows)
}

fn is_perfect_square(len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let s = (len as f64).sqrt() as usize;
    (s * s == len).then_some(s)
}

/// Square 2×2 / 3×3 from surface numbers, `data-n`, or demo SPD [[2,1],[1,2]].
fn resolve_square23(document: &Document) -> (usize, Vec<f64>) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let n_attr = usize_attr(container.as_ref(), "data-n");
    if let Some(n) = n_attr.filter(|n| *n == 2 || *n == 3) {
        if nums.len() >= n * n {
            return (n, nums[..n * n].to_vec());
        }
    }
    if let Some(n) = is_perfect_square(nums.len()).filter(|n| *n == 2 || *n == 3) {
        return (n, nums);
    }
    // Prefer 2×2 when 4 nums + vector tail for gaussian; else demo.
    if nums.len() >= 4 && nums.len() < 9 {
        return (2, nums[..4].to_vec());
    }
    if nums.len() >= 9 {
        return (3, nums[..9].to_vec());
    }
    (2, vec![2.0, 1.0, 1.0, 2.0])
}

fn local_gauss2(a: &[f64], b: &[f64]) -> Option<[f64; 2]> {
    if a.len() < 4 || b.len() < 2 {
        return None;
    }
    let det = a[0] * a[3] - a[1] * a[2];
    if !(det.abs() > 1e-15) {
        return None;
    }
    let x0 = (b[0] * a[3] - a[1] * b[1]) / det;
    let x1 = (a[0] * b[1] - b[0] * a[2]) / det;
    (x0.is_finite() && x1.is_finite()).then_some([x0, x1])
}

fn local_jacobi2(a: &[f64]) -> Option<[f64; 2]> {
    if a.len() < 4 {
        return None;
    }
    let (aa, bb, cc) = (a[0], a[1], a[3]);
    let tr = aa + cc;
    let det = aa * cc - bb * bb;
    let disc = (tr * tr - 4.0 * det).max(0.0).sqrt();
    let e0 = 0.5 * (tr + disc);
    let e1 = 0.5 * (tr - disc);
    (e0.is_finite() && e1.is_finite()).then_some([e0, e1])
}

fn local_transpose(n: usize, a: &[f64]) -> Vec<f64> {
    let mut t = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            t[j * n + i] = a[i * n + j];
        }
    }
    t
}

/// Identity Löwdin sketch: X≈I when S≈I; else report diag S^{-1/2} only.
fn local_lowdin_sketch(n: usize, s: &[f64]) -> Option<String> {
    if s.len() < n * n {
        return None;
    }
    let mut off = 0.0;
    for i in 0..n {
        for j in 0..n {
            if i != j {
                off += s[i * n + j].abs();
            }
        }
    }
    if off < 1e-9 {
        let mut inv_sqrt = Vec::with_capacity(n);
        for i in 0..n {
            let d = s[i * n + i];
            if !(d > 0.0) {
                return None;
            }
            inv_sqrt.push(1.0 / d.sqrt());
        }
        return Some(format!("Löwdin X≈diag({inv_sqrt:?}) for diagonal S"));
    }
    Some(format!("Löwdin S^{{-1/2}} sketch for {n}×{n} (live for full Jacobi path)"))
}

const ELEMENT_SYMBOLS: &[&str] = &[
    "", "H", "He", "Li", "Be", "B", "C", "N", "O", "F", "Ne", "Na", "Mg", "Al", "Si", "P",
    "S", "Cl", "Ar", "K", "Ca", "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn",
    "Ga", "Ge", "As", "Se", "Br", "Kr",
];

fn symbol_for_z(z: u64) -> Option<&'static str> {
    ELEMENT_SYMBOLS.get(z as usize).copied().filter(|s| !s.is_empty())
}

fn z_for_symbol(sym: &str) -> Option<u64> {
    let s = sym.trim();
    ELEMENT_SYMBOLS
        .iter()
        .enumerate()
        .find(|(_, e)| e.eq_ignore_ascii_case(s))
        .map(|(i, _)| i as u64)
        .filter(|z| *z > 0)
}

fn weight_for_symbol(sym: &str) -> Option<f64> {
    // IUPAC-ish common weights for dual-path sketch (Host is authoritative).
    Some(match sym.trim().to_ascii_lowercase().as_str() {
        "h" => 1.008,
        "he" => 4.0026,
        "c" => 12.011,
        "n" => 14.007,
        "o" => 15.999,
        "f" => 18.998,
        "ne" => 20.180,
        "na" => 22.990,
        "mg" => 24.305,
        "si" => 28.085,
        "p" => 30.974,
        "s" => 32.06,
        "cl" => 35.45,
        "ar" => 39.948,
        "k" => 39.098,
        "ca" => 40.078,
        "fe" => 55.845,
        "cu" => 63.546,
        "zn" => 65.38,
        "br" => 79.904,
        _ => return None,
    })
}

/// Slater LDA exchange sketch: ε_x = −¾(3/π)^{1/3} ρ^{1/3}, v_x = 4/3 ε_x.
fn local_lda_exchange(rho: f64) -> Option<(f64, f64)> {
    if !(rho > 0.0) || !rho.is_finite() {
        return None;
    }
    let cx = 0.75 * (3.0 / std::f64::consts::PI).powf(1.0 / 3.0);
    let ex = -cx * rho.powf(1.0 / 3.0);
    let vx = (4.0 / 3.0) * ex;
    (ex.is_finite() && vx.is_finite()).then_some((ex, vx))
}

/// `Chemistry.gaussian_elimination` — `{ a, b }` for N∈{2,3}.
pub(super) fn run_gaussian_elimination(document: &Document, label: &str) {
    let (n, a) = resolve_square23(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let b: Vec<f64> = if nums.len() >= n * n + n {
        nums[n * n..n * n + n].to_vec()
    } else if n == 2 {
        vec![4.0, 5.0]
    } else {
        vec![1.0, 2.0, 3.0]
    };
    let sketch = if n == 2 {
        local_gauss2(&a, &b).map(|x| format!("gaussian_elimination sketch x={x:?}"))
    } else {
        Some(format!(
            "gaussian_elimination sketch {n}×{n} (live for Host GE)"
        ))
    };
    let Some(msg) = sketch else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need nonsingular 2×2/3×3 A then b (surface numbers or data-n).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Chemistry.gaussian_elimination",
        msg,
        json!({ "a": nested_square(n, &a), "b": b }),
    );
}

/// `Chemistry.jacobi_diagonalization` — `{ a }` symmetric N∈{2,3}.
pub(super) fn run_jacobi_diagonalization(document: &Document, label: &str) {
    let (n, a) = resolve_square23(document);
    let sketch = if n == 2 {
        local_jacobi2(&a).map(|e| format!("jacobi sketch eigenvalues≈{e:?}"))
    } else {
        Some(format!(
            "jacobi_diagonalization sketch {n}×{n} (live for Host Jacobi)"
        ))
    };
    let Some(msg) = sketch else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need 2×2/3×3 symmetric matrix (surface or data-n).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Chemistry.jacobi_diagonalization",
        msg,
        json!({ "a": nested_square(n, &a) }),
    );
}

/// `Chemistry.transpose` — `{ a }` square N∈{2,3}.
pub(super) fn run_transpose(document: &Document, label: &str) {
    let (n, a) = resolve_square23(document);
    let t = local_transpose(n, &a);
    invoke_dual(
        document,
        label,
        "Chemistry.transpose",
        format!("transpose sketch {n}×{n} → {t:?}"),
        json!({ "a": nested_square(n, &a) }),
    );
}

/// `Chemistry.orthogonalization_matrix` — `{ s }` overlap N∈{2,3}.
pub(super) fn run_orthogonalization_matrix(document: &Document, label: &str) {
    let (n, s) = resolve_square23(document);
    let Some(msg) = local_lowdin_sketch(n, &s) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need SPD 2×2/3×3 overlap S (surface or data-n).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Chemistry.orthogonalization_matrix",
        msg,
        json!({ "s": nested_square(n, &s) }),
    );
}

/// `Chemistry.element_symbol` — `{ atomic_number }`.
pub(super) fn run_element_symbol(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let z = usize_attr(container.as_ref(), "data-z")
        .or_else(|| nums.first().map(|v| *v as usize))
        .unwrap_or(6) as u64;
    let sketch = match symbol_for_z(z) {
        Some(sym) => format!("element_symbol sketch Z={z} → {sym}"),
        None => format!("element_symbol sketch Z={z} (live for full table)"),
    };
    invoke_dual(
        document,
        label,
        "Chemistry.element_symbol",
        sketch,
        json!({ "atomic_number": z }),
    );
}

/// `Chemistry.atomic_number` — `{ symbol }`.
pub(super) fn run_atomic_number(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let sym = container
        .as_ref()
        .and_then(|e| e.get_attribute("data-symbol"))
        .or_else(|| {
            source
                .split_whitespace()
                .find(|t| t.chars().all(|c| c.is_ascii_alphabetic()))
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "C".into());
    let sketch = match z_for_symbol(&sym) {
        Some(z) => format!("atomic_number sketch '{sym}' → Z={z}"),
        None => format!("atomic_number sketch '{sym}' (live for Host table)"),
    };
    invoke_dual(
        document,
        label,
        "Chemistry.atomic_number",
        sketch,
        json!({ "symbol": sym }),
    );
}

/// `Chemistry.standard_atomic_weight` — `{ element }`.
pub(super) fn run_standard_atomic_weight(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let elem = container
        .as_ref()
        .and_then(|e| e.get_attribute("data-element"))
        .or_else(|| {
            container
                .as_ref()
                .and_then(|e| e.get_attribute("data-symbol"))
        })
        .or_else(|| {
            source
                .split_whitespace()
                .find(|t| t.chars().all(|c| c.is_ascii_alphabetic()))
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "C".into());
    let sketch = match weight_for_symbol(&elem) {
        Some(w) => format!("standard_atomic_weight sketch {elem} ≈ {w}"),
        None => format!("standard_atomic_weight sketch {elem} (live for Host)"),
    };
    invoke_dual(
        document,
        label,
        "Chemistry.standard_atomic_weight",
        sketch,
        json!({ "element": elem }),
    );
}

/// `Chemistry.lda_exchange` — `{ rho }`.
pub(super) fn run_lda_exchange(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let rho = numeric_attr(container.as_ref(), "data-rho")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.1);
    let Some((ex, vx)) = local_lda_exchange(rho) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need rho > 0 (data-rho or one surface number).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Chemistry.lda_exchange",
        format!("LDA exchange sketch ρ={rho} → ε≈{ex:.6}, v≈{vx:.6}"),
        json!({ "rho": rho }),
    );
}

/// `Chemistry.lda_correlation_vwn` — `{ rho }`.
pub(super) fn run_lda_correlation_vwn(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let rho = numeric_attr(container.as_ref(), "data-rho")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.1);
    if !(rho > 0.0) || !rho.is_finite() {
        super::interactions::show_tool_status(
            document,
            label,
            "Need rho > 0 (data-rho or one surface number).",
            "error",
        );
        return;
    }
    invoke_dual(
        document,
        label,
        "Chemistry.lda_correlation_vwn",
        format!("LDA VWN correlation sketch ρ={rho} (live for Host VWN)"),
        json!({ "rho": rho }),
    );
}

/// `Chemistry.sto3g_h2` — no args; Host returns STO-3G H₂ summary.
pub(super) fn run_sto3g_h2(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Chemistry.sto3g_h2",
        "STO-3G H₂ summary sketch E≈−1.117 Eh at R=1.4 a₀".into(),
        json!({}),
    );
}
