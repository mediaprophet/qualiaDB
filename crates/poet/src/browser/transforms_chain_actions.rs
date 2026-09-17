//! Dual-path Tool Chest actions for curated `IntegralTransforms.*` ALL_BOUND ids (wave 19).
//!
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.
//! Local sketches mirror Host DFT / Z / Laplace algebra (CPU; Host path authoritative).

use serde_json::json;
use web_sys::{Document, Element};

type Cplx = (f64, f64);

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
        .take(256)
        .collect()
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn string_attr(el: Option<&Element>, name: &str) -> Option<String> {
    el.and_then(|e| e.get_attribute(name))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
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

fn cadd(a: Cplx, b: Cplx) -> Cplx {
    (a.0 + b.0, a.1 + b.1)
}

fn cmul(a: Cplx, b: Cplx) -> Cplx {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

fn cinv(z: Cplx) -> Option<Cplx> {
    let d = z.0 * z.0 + z.1 * z.1;
    if d == 0.0 {
        return None;
    }
    Some((z.0 / d, -z.1 / d))
}

fn local_dft(x: &[Cplx]) -> Vec<Cplx> {
    let n = x.len();
    let mut out = vec![(0.0, 0.0); n];
    if n == 0 {
        return out;
    }
    let w = -2.0 * std::f64::consts::PI / n as f64;
    for (k, ok) in out.iter_mut().enumerate() {
        let mut acc = (0.0, 0.0);
        for (j, &xj) in x.iter().enumerate() {
            let ang = w * (k * j) as f64;
            acc = cadd(acc, cmul(xj, (ang.cos(), ang.sin())));
        }
        *ok = acc;
    }
    out
}

fn local_idft(spectrum: &[Cplx]) -> Vec<Cplx> {
    let n = spectrum.len();
    let mut out = vec![(0.0, 0.0); n];
    if n == 0 {
        return out;
    }
    let w = 2.0 * std::f64::consts::PI / n as f64;
    let inv = 1.0 / n as f64;
    for (j, oj) in out.iter_mut().enumerate() {
        let mut acc = (0.0, 0.0);
        for (k, &xk) in spectrum.iter().enumerate() {
            let ang = w * (k * j) as f64;
            acc = cadd(acc, cmul(xk, (ang.cos(), ang.sin())));
        }
        *oj = (acc.0 * inv, acc.1 * inv);
    }
    out
}

fn local_z_transform_finite(x: &[f64], z: Cplx) -> Option<Cplx> {
    let zinv = cinv(z)?;
    let mut acc = (0.0, 0.0);
    let mut zpow = (1.0, 0.0);
    for &xn in x {
        acc = (acc.0 + xn * zpow.0, acc.1 + xn * zpow.1);
        zpow = cmul(zpow, zinv);
    }
    Some(acc)
}

fn local_unit_step_z(z: Cplx) -> Option<Cplx> {
    let zinv = cinv(z)?;
    let denom = (1.0 - zinv.0, -zinv.1);
    cinv(denom)
}

fn local_geometric_z(a: f64, z: Cplx) -> Option<Cplx> {
    let zinv = cinv(z)?;
    let azinv = (a * zinv.0, a * zinv.1);
    let denom = (1.0 - azinv.0, -azinv.1);
    cinv(denom)
}

/// Trapezoidal sketch of ∫₀^{t_max} e^{−s t} f(t) dt for a few named exprs.
fn local_laplace_numeric(expr: &str, s: f64, t_max: f64, steps: usize) -> Option<f64> {
    if !(s > 0.0) || !(t_max > 0.0) || steps < 2 {
        return None;
    }
    let steps = if steps % 2 == 0 { steps } else { steps + 1 };
    let f: fn(f64) -> f64 = match expr.trim().to_ascii_lowercase().as_str() {
        "1" | "one" => |_| 1.0,
        "t" => |t| t,
        "exp(-t)" | "e^(-t)" => |t| (-t).exp(),
        _ => return None,
    };
    let h = t_max / steps as f64;
    let mut acc = 0.0;
    for i in 0..=steps {
        let t = i as f64 * h;
        let w = if i == 0 || i == steps {
            0.5
        } else {
            1.0
        };
        acc += w * (-s * t).exp() * f(t);
    }
    Some(acc * h)
}

fn local_laplace_symbolic(expr: &str) -> Option<&'static str> {
    match expr.trim().to_ascii_lowercase().as_str() {
        "1" | "one" => Some("1/s"),
        "t" => Some("1/s^2"),
        "exp(-t)" | "e^(-t)" => Some("1/(s+1)"),
        "sin(t)" => Some("1/(s^2+1)"),
        "cos(t)" => Some("s/(s^2+1)"),
        _ => None,
    }
}

fn pairs_from_numbers(nums: &[f64]) -> Vec<Cplx> {
    nums.chunks(2)
        .filter_map(|c| {
            if c.len() == 2 {
                Some((c[0], c[1]))
            } else {
                None
            }
        })
        .take(128)
        .collect()
}

fn resolve_z(document: &Document, nums: &[f64], default: Cplx) -> Cplx {
    let container = selected_container(document);
    let re = numeric_attr(container.as_ref(), "data-z-re")
        .or_else(|| numeric_attr(container.as_ref(), "data-re"))
        .or_else(|| {
            if nums.len() >= 2 {
                Some(nums[nums.len() - 2])
            } else {
                nums.first().copied()
            }
        })
        .unwrap_or(default.0);
    let im = numeric_attr(container.as_ref(), "data-z-im")
        .or_else(|| numeric_attr(container.as_ref(), "data-im"))
        .or_else(|| {
            if nums.len() >= 2 {
                Some(nums[nums.len() - 1])
            } else {
                None
            }
        })
        .unwrap_or(default.1);
    (re, im)
}

fn format_cplx(c: Cplx) -> String {
    format!("({:.6}, {:.6})", c.0, c.1)
}

fn mag0(spectrum: &[Cplx]) -> f64 {
    spectrum
        .first()
        .map(|c| (c.0 * c.0 + c.1 * c.1).sqrt())
        .unwrap_or(0.0)
}

/// `IntegralTransforms.dft` — real signal from surface numbers (`data` list).
pub(super) fn run_dft(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let data = if nums.is_empty() {
        vec![1.0, 1.0, 1.0, 1.0]
    } else {
        nums
    };
    let x: Vec<Cplx> = data.iter().map(|&r| (r, 0.0)).collect();
    let spectrum = local_dft(&x);
    let sketch = format!(
        "DFT(N={}) sketch |X[0]|≈{:.6}",
        spectrum.len(),
        mag0(&spectrum)
    );
    invoke_dual(
        document,
        label,
        "IntegralTransforms.dft",
        sketch,
        json!({ "data": data }),
    );
}

/// `IntegralTransforms.dft_complex` — complex samples as consecutive [re,im] pairs.
pub(super) fn run_dft_complex(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let samples = {
        let pairs = pairs_from_numbers(&nums);
        if pairs.is_empty() {
            vec![(1.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0)]
        } else {
            pairs
        }
    };
    let spectrum = local_dft(&samples);
    let sketch = format!(
        "DFT_complex(N={}) sketch |X[0]|≈{:.6}",
        spectrum.len(),
        mag0(&spectrum)
    );
    let sample_lists: Vec<Vec<f64>> = samples.iter().map(|(re, im)| vec![*re, *im]).collect();
    invoke_dual(
        document,
        label,
        "IntegralTransforms.dft_complex",
        sketch,
        json!({ "samples": sample_lists }),
    );
}

/// `IntegralTransforms.idft` — spectrum as consecutive [re,im] pairs.
pub(super) fn run_idft(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let spectrum = {
        let pairs = pairs_from_numbers(&nums);
        if pairs.is_empty() {
            vec![(4.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0)]
        } else {
            pairs
        }
    };
    let time = local_idft(&spectrum);
    let sketch = format!(
        "IDFT(N={}) sketch x[0]≈{}",
        time.len(),
        format_cplx(time.first().copied().unwrap_or((0.0, 0.0)))
    );
    let spectrum_lists: Vec<Vec<f64>> = spectrum.iter().map(|(re, im)| vec![*re, *im]).collect();
    invoke_dual(
        document,
        label,
        "IntegralTransforms.idft",
        sketch,
        json!({ "spectrum": spectrum_lists }),
    );
}

/// `IntegralTransforms.z_transform_finite` — sequence + complex z.
pub(super) fn run_z_transform_finite(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let z = resolve_z(document, &nums, (2.0, 0.0));
    let has_z_attr = numeric_attr(container.as_ref(), "data-z-re").is_some()
        || numeric_attr(container.as_ref(), "data-re").is_some();
    let x = if has_z_attr {
        if nums.is_empty() {
            vec![1.0, 2.0, 3.0]
        } else {
            nums
        }
    } else if nums.len() >= 3 {
        nums[..nums.len() - 2].to_vec()
    } else if nums.is_empty() {
        vec![1.0, 2.0, 3.0]
    } else {
        nums
    };
    let Some(result) = local_z_transform_finite(&x, z) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need finite x and z≠0 (attrs data-z-re/data-z-im).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "IntegralTransforms.z_transform_finite",
        format!(
            "Z{{x}}(z={}) sketch ≈ {}",
            format_cplx(z),
            format_cplx(result)
        ),
        json!({ "x": x, "z": [z.0, z.1] }),
    );
}

/// `IntegralTransforms.unit_step_z` — closed form at complex z (|z|>1).
pub(super) fn run_unit_step_z(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let z = resolve_z(document, &nums, (2.0, 0.0));
    let Some(result) = local_unit_step_z(z) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need z with |z|>1 and z≠1 (attrs data-z-re/data-z-im).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "IntegralTransforms.unit_step_z",
        format!(
            "Z{{u}}(z={}) sketch ≈ {}",
            format_cplx(z),
            format_cplx(result)
        ),
        json!({ "z": [z.0, z.1] }),
    );
}

/// `IntegralTransforms.geometric_z` — a^n at complex z.
pub(super) fn run_geometric_z(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let a = numeric_attr(container.as_ref(), "data-a")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.5);
    let z = {
        let re = numeric_attr(container.as_ref(), "data-z-re")
            .or_else(|| numeric_attr(container.as_ref(), "data-re"))
            .or_else(|| nums.get(1).copied())
            .unwrap_or(2.0);
        let im = numeric_attr(container.as_ref(), "data-z-im")
            .or_else(|| numeric_attr(container.as_ref(), "data-im"))
            .or_else(|| nums.get(2).copied())
            .unwrap_or(0.0);
        (re, im)
    };
    let Some(result) = local_geometric_z(a, z) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a and z≠0 with |z|>|a| (attrs data-a/data-z-re/data-z-im).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "IntegralTransforms.geometric_z",
        format!(
            "Z{{a^n}}(a={a}, z={}) sketch ≈ {}",
            format_cplx(z),
            format_cplx(result)
        ),
        json!({ "a": a, "z": [z.0, z.1] }),
    );
}

/// `IntegralTransforms.laplace_numeric` — expr + s (+ optional t_max/steps).
pub(super) fn run_laplace_numeric(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let expr = string_attr(container.as_ref(), "data-expr")
        .or_else(|| {
            source
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty() && l.parse::<f64>().is_err())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "1".into());
    let nums = parse_numbers(&source);
    let s = numeric_attr(container.as_ref(), "data-s")
        .or_else(|| nums.first().copied())
        .unwrap_or(1.0);
    let t_max = numeric_attr(container.as_ref(), "data-t-max").unwrap_or(10.0);
    let steps = numeric_attr(container.as_ref(), "data-steps")
        .map(|v| v.max(2.0) as i64)
        .unwrap_or(100);
    let Some(sketch_val) = local_laplace_numeric(&expr, s, t_max, steps as usize) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need expr in {1,t,exp(-t)} and s>0 (attrs data-expr/data-s).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "IntegralTransforms.laplace_numeric",
        format!("L{{{expr}}}(s={s}) numeric sketch ≈ {sketch_val:.6}"),
        json!({ "expr": expr, "s": s, "t_max": t_max, "steps": steps }),
    );
}

/// `IntegralTransforms.laplace_symbolic` — algebraic table lookup sketch.
pub(super) fn run_laplace_symbolic(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let expr = string_attr(container.as_ref(), "data-expr")
        .or_else(|| {
            source
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "1".into());
    let Some(transform) = local_laplace_symbolic(&expr) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a table expr (1, t, exp(-t), sin(t), cos(t)) via data-expr.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "IntegralTransforms.laplace_symbolic",
        format!("L{{{expr}}} symbolic sketch → {transform}"),
        json!({ "expr": expr }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_it_wave19_transforms_match_known() {
        // DFT([1,1,1,1]) → DC bin 4.
        let spectrum = local_dft(&[(1.0, 0.0), (1.0, 0.0), (1.0, 0.0), (1.0, 0.0)]);
        assert!((spectrum[0].0 - 4.0).abs() < 1e-9);
        assert!(spectrum[0].1.abs() < 1e-9);
        for k in 1..4 {
            assert!(spectrum[k].0.abs() < 1e-9);
            assert!(spectrum[k].1.abs() < 1e-9);
        }

        // Impulse → flat spectrum magnitude 1.
        let impulse = local_dft(&[(1.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0)]);
        for c in &impulse {
            assert!((c.0 - 1.0).abs() < 1e-9);
            assert!(c.1.abs() < 1e-9);
        }

        let time = local_idft(&[(4.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0)]);
        for c in &time {
            assert!((c.0 - 1.0).abs() < 1e-9);
            assert!(c.1.abs() < 1e-9);
        }

        let z = local_z_transform_finite(&[1.0, 2.0, 3.0], (2.0, 0.0)).unwrap();
        assert!((z.0 - 2.75).abs() < 1e-9);

        let us = local_unit_step_z((2.0, 0.0)).unwrap();
        assert!((us.0 - 2.0).abs() < 1e-9); // z/(z-1) at z=2 → 2

        let g = local_geometric_z(0.5, (2.0, 0.0)).unwrap();
        assert!((g.0 - (1.0 / (1.0 - 0.5 / 2.0))).abs() < 1e-9);

        let ln = local_laplace_numeric("1", 1.0, 20.0, 200).unwrap();
        assert!((ln - 1.0).abs() < 0.05); // ≈ 1/s = 1

        assert_eq!(local_laplace_symbolic("t"), Some("1/s^2"));
        assert_eq!(local_laplace_symbolic("exp(-t)"), Some("1/(s+1)"));
    }
}
