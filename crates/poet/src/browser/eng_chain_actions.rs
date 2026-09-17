//! Dual-path Tool Chest actions for curated `EngineeringAnalysis.*` ALL_BOUND ids (wave 17).
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

fn nth(nums: &[f64], i: usize, default: f64) -> f64 {
    nums.get(i).copied().unwrap_or(default)
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

fn sketch_natural_frequency(stiffness: f64, mass: f64) -> Option<f64> {
    if mass <= 0.0 || stiffness < 0.0 {
        return None;
    }
    Some((stiffness / mass).sqrt())
}

fn sketch_harmonic_sdof(
    mass: f64,
    damping: f64,
    stiffness: f64,
    force_amplitude: f64,
    freqs: &[f64],
) -> Option<(Vec<f64>, Vec<f64>)> {
    if mass <= 0.0 || damping < 0.0 || stiffness < 0.0 || freqs.is_empty() {
        return None;
    }
    let mut amps = Vec::with_capacity(freqs.len());
    let mut phases = Vec::with_capacity(freqs.len());
    for &w in freqs {
        let re = stiffness - mass * w * w;
        let im = damping * w;
        let denom = (re * re + im * im).sqrt();
        amps.push(if denom > 0.0 {
            force_amplitude / denom
        } else {
            f64::INFINITY
        });
        phases.push(im.atan2(re));
    }
    Some((amps, phases))
}

fn sketch_euler(e: f64, i: f64, length: f64, k: f64, num_modes: usize) -> Option<Vec<f64>> {
    if e <= 0.0 || i <= 0.0 || length <= 0.0 || k <= 0.0 || num_modes == 0 {
        return None;
    }
    let le = k * length;
    let base = std::f64::consts::PI.powi(2) * e * i / (le * le);
    Some((1..=num_modes).map(|n| (n as f64).powi(2) * base).collect())
}

/// Acklam rational approximation of Φ⁻¹ (same family as Host reliability).
fn sketch_inv_norm(p: f64) -> f64 {
    let p = p.clamp(1e-12, 1.0 - 1e-12);
    let a = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    let b = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_607e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    let c = [
        -7.784_894_002_430_293e-3,
        -3.223_726_638_814_55e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    let d = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    let plow = 0.02425;
    let phigh = 1.0 - plow;
    if p < plow {
        let q = (-2.0 * p.ln()).sqrt();
        (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    } else if p > phigh {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    } else {
        let q = p - 0.5;
        let r = q * q;
        (((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q
            / (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1.0)
    }
}

fn sketch_reliability_index(failure_prob: f64) -> f64 {
    -sketch_inv_norm(failure_prob)
}

fn sketch_kinematics(x0: f64, v0: f64, a: f64, times: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut positions = Vec::with_capacity(times.len());
    let mut velocities = Vec::with_capacity(times.len());
    for &t in times {
        positions.push(x0 + v0 * t + 0.5 * a * t * t);
        velocities.push(v0 + a * t);
    }
    (positions, velocities)
}

fn sketch_von_mises(tensor: &[f64; 9]) -> f64 {
    let (sxx, syy, szz) = (tensor[0], tensor[4], tensor[8]);
    let (txy, tyz, tzx) = (tensor[1], tensor[5], tensor[6]);
    (0.5 * ((sxx - syy).powi(2) + (syy - szz).powi(2) + (szz - sxx).powi(2))
        + 3.0 * (txy * txy + tyz * tyz + tzx * tzx))
        .sqrt()
}

fn sketch_drag(rho: f64, v: f64, cd: f64, area: f64) -> f64 {
    0.5 * rho * v * v * cd * area
}

fn sketch_reynolds(density: f64, velocity: f64, char_length: f64, viscosity: f64) -> f64 {
    if viscosity == 0.0 {
        f64::INFINITY
    } else {
        density * velocity * char_length / viscosity
    }
}

fn sketch_fatigue(sa: f64, sf: f64, b: f64) -> f64 {
    if sa <= 0.0 || sf <= 0.0 || b == 0.0 {
        f64::INFINITY
    } else {
        0.5 * (sa / sf).powf(1.0 / b)
    }
}

fn sketch_miner(blocks: &[(f64, f64)]) -> f64 {
    let mut d = 0.0;
    for &(applied, allowable) in blocks {
        if allowable > 0.0 {
            d += applied / allowable;
        }
    }
    d
}

/// `EngineeringAnalysis.natural_frequency_sdof`
pub(super) fn run_natural_frequency_sdof(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let stiffness =
        numeric_attr(container.as_ref(), "data-stiffness").unwrap_or_else(|| nth(&nums, 0, 4.0));
    let mass = numeric_attr(container.as_ref(), "data-mass").unwrap_or_else(|| nth(&nums, 1, 1.0));
    let local = match sketch_natural_frequency(stiffness, mass) {
        Some(omega) => format!("SDOF ωₙ sketch ≈ {omega} rad/s"),
        None => "SDOF ωₙ sketch: need mass>0, stiffness≥0".into(),
    };
    invoke_dual(
        document,
        label,
        "EngineeringAnalysis.natural_frequency_sdof",
        local,
        json!({ "stiffness": stiffness, "mass": mass }),
    );
}

/// `EngineeringAnalysis.analyze_harmonic_sdof`
pub(super) fn run_analyze_harmonic_sdof(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let mass = numeric_attr(container.as_ref(), "data-mass").unwrap_or_else(|| nth(&nums, 0, 1.0));
    let damping =
        numeric_attr(container.as_ref(), "data-damping").unwrap_or_else(|| nth(&nums, 1, 0.0));
    let stiffness =
        numeric_attr(container.as_ref(), "data-stiffness").unwrap_or_else(|| nth(&nums, 2, 4.0));
    let force_amplitude = numeric_attr(container.as_ref(), "data-force")
        .unwrap_or_else(|| nth(&nums, 3, 2.0));
    let freqs: Vec<f64> = if nums.len() > 4 {
        nums[4..].to_vec()
    } else {
        vec![numeric_attr(container.as_ref(), "data-omega").unwrap_or(0.0)]
    };
    let local = match sketch_harmonic_sdof(mass, damping, stiffness, force_amplitude, &freqs) {
        Some((amps, _)) => format!(
            "Harmonic SDOF sketch X[0]≈{}",
            amps.first().copied().unwrap_or(f64::NAN)
        ),
        None => "Harmonic SDOF sketch: invalid mass/damping/stiffness/freqs".into(),
    };
    invoke_dual(
        document,
        label,
        "EngineeringAnalysis.analyze_harmonic_sdof",
        local,
        json!({
            "mass": mass,
            "damping": damping,
            "stiffness": stiffness,
            "force_amplitude": force_amplitude,
            "excitation_freqs": freqs,
        }),
    );
}

/// `EngineeringAnalysis.analyze_euler`
pub(super) fn run_analyze_euler(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let e = numeric_attr(container.as_ref(), "data-e").unwrap_or_else(|| nth(&nums, 0, 1.0));
    let i = numeric_attr(container.as_ref(), "data-i").unwrap_or_else(|| nth(&nums, 1, 1.0));
    let length =
        numeric_attr(container.as_ref(), "data-length").unwrap_or_else(|| nth(&nums, 2, 1.0));
    let k = numeric_attr(container.as_ref(), "data-k").unwrap_or_else(|| nth(&nums, 3, 1.0));
    let num_modes = usize_attr(container.as_ref(), "data-modes")
        .or_else(|| nums.get(4).map(|v| (*v as usize).max(1)))
        .unwrap_or(1)
        .clamp(1, 8);
    let local = match sketch_euler(e, i, length, k, num_modes) {
        Some(loads) => format!(
            "Euler buckling sketch P_cr[1]≈{}",
            loads.first().copied().unwrap_or(f64::NAN)
        ),
        None => "Euler buckling sketch: need positive E,I,L,K and modes≥1".into(),
    };
    invoke_dual(
        document,
        label,
        "EngineeringAnalysis.analyze_euler",
        local,
        json!({
            "youngs_modulus": e,
            "moment_of_inertia": i,
            "length": length,
            "effective_length_factor": k,
            "num_modes": num_modes as u64,
        }),
    );
}

/// `EngineeringAnalysis.compute_reliability_index`
pub(super) fn run_compute_reliability_index(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let p =
        numeric_attr(container.as_ref(), "data-pf").unwrap_or_else(|| nth(&nums, 0, 0.5));
    let beta = sketch_reliability_index(p);
    invoke_dual(
        document,
        label,
        "EngineeringAnalysis.compute_reliability_index",
        format!("Reliability index sketch β≈{beta} (p_f={p})"),
        json!({ "failure_prob": p }),
    );
}

/// `EngineeringAnalysis.kinematics`
pub(super) fn run_kinematics(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let x0 = numeric_attr(container.as_ref(), "data-x0").unwrap_or_else(|| nth(&nums, 0, 0.0));
    let v0 = numeric_attr(container.as_ref(), "data-v0").unwrap_or_else(|| nth(&nums, 1, 0.0));
    let a = numeric_attr(container.as_ref(), "data-a").unwrap_or_else(|| nth(&nums, 2, 0.0));
    let times: Vec<f64> = if nums.len() > 3 {
        nums[3..].to_vec()
    } else {
        vec![0.0, 1.0]
    };
    let (pos, vel) = sketch_kinematics(x0, v0, a, &times);
    let local = format!(
        "Kinematics sketch x[last]≈{}, v[last]≈{}",
        pos.last().copied().unwrap_or(0.0),
        vel.last().copied().unwrap_or(0.0)
    );
    invoke_dual(
        document,
        label,
        "EngineeringAnalysis.kinematics",
        local,
        json!({ "x0": x0, "v0": v0, "a": a, "t": times }),
    );
}

/// `EngineeringAnalysis.cauchy_stress`
pub(super) fn run_cauchy_stress(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let mut tensor = [0.0f64; 9];
    if nums.len() >= 9 {
        tensor.copy_from_slice(&nums[..9]);
    } else if let Some(sxx) = numeric_attr(container.as_ref(), "data-sxx") {
        tensor[0] = sxx;
    } else {
        tensor[0] = 100.0; // default uniaxial
    }
    let vm = sketch_von_mises(&tensor);
    let hydro = (tensor[0] + tensor[4] + tensor[8]) / 3.0;
    invoke_dual(
        document,
        label,
        "EngineeringAnalysis.cauchy_stress",
        format!("Cauchy stress sketch von_Mises≈{vm}, hydro≈{hydro}"),
        json!({ "tensor": tensor.to_vec() }),
    );
}

/// `EngineeringAnalysis.drag_force`
pub(super) fn run_drag_force(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let rho =
        numeric_attr(container.as_ref(), "data-rho").unwrap_or_else(|| nth(&nums, 0, 1.225));
    let v = numeric_attr(container.as_ref(), "data-v").unwrap_or_else(|| nth(&nums, 1, 10.0));
    let cd = numeric_attr(container.as_ref(), "data-cd").unwrap_or_else(|| nth(&nums, 2, 1.0));
    let area =
        numeric_attr(container.as_ref(), "data-area").unwrap_or_else(|| nth(&nums, 3, 1.0));
    let force = sketch_drag(rho, v, cd, area);
    invoke_dual(
        document,
        label,
        "EngineeringAnalysis.drag_force",
        format!("Drag force sketch F≈{force} N"),
        json!({
            "air_density": rho,
            "velocity": v,
            "drag_coefficient": cd,
            "area": area,
        }),
    );
}

/// `EngineeringAnalysis.reynolds_number`
pub(super) fn run_reynolds_number(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let density =
        numeric_attr(container.as_ref(), "data-density").unwrap_or_else(|| nth(&nums, 0, 1.225));
    let velocity =
        numeric_attr(container.as_ref(), "data-velocity").unwrap_or_else(|| nth(&nums, 1, 10.0));
    let char_length =
        numeric_attr(container.as_ref(), "data-length").unwrap_or_else(|| nth(&nums, 2, 1.0));
    let viscosity = numeric_attr(container.as_ref(), "data-mu")
        .unwrap_or_else(|| nth(&nums, 3, 1.81e-5));
    let re = sketch_reynolds(density, velocity, char_length, viscosity);
    invoke_dual(
        document,
        label,
        "EngineeringAnalysis.reynolds_number",
        format!("Reynolds sketch Re≈{re}"),
        json!({
            "density": density,
            "velocity": velocity,
            "char_length": char_length,
            "dynamic_viscosity": viscosity,
        }),
    );
}

/// `EngineeringAnalysis.fatigue_cycles`
pub(super) fn run_fatigue_cycles(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let sa =
        numeric_attr(container.as_ref(), "data-sa").unwrap_or_else(|| nth(&nums, 0, 200.0));
    let sf =
        numeric_attr(container.as_ref(), "data-sf").unwrap_or_else(|| nth(&nums, 1, 900.0));
    let b = numeric_attr(container.as_ref(), "data-b").unwrap_or_else(|| nth(&nums, 2, -0.1));
    let n = sketch_fatigue(sa, sf, b);
    invoke_dual(
        document,
        label,
        "EngineeringAnalysis.fatigue_cycles",
        format!("Basquin fatigue sketch N≈{n}"),
        json!({
            "stress_amplitude": sa,
            "fatigue_strength_coeff": sf,
            "fatigue_strength_exponent": b,
        }),
    );
}

/// `EngineeringAnalysis.miner_damage`
pub(super) fn run_miner_damage(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let _ = container;
    let flat: Vec<f64> = if nums.len() >= 2 && nums.len() % 2 == 0 {
        nums
    } else {
        vec![100.0, 1000.0, 50.0, 2000.0]
    };
    let blocks: Vec<(f64, f64)> = flat.chunks(2).map(|c| (c[0], c[1])).collect();
    let damage = sketch_miner(&blocks);
    invoke_dual(
        document,
        label,
        "EngineeringAnalysis.miner_damage",
        format!("Miner damage sketch D≈{damage}"),
        json!({ "blocks": flat }),
    );
}
