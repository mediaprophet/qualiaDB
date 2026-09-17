//! Dual-path Tool Chest actions for curated `Physics.*` ALL_BOUND ids (waves 14–15).
//!
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.

use serde_json::json;
use web_sys::{Document, Element};

const C_LIGHT: f64 = 299_792_458.0;

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
        if v.is_finite() && v >= 1.0 && v == v.floor() {
            Some(v as usize)
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

fn local_doppler(f_src: f64, v: f64, c: f64) -> Option<(f64, f64, f64)> {
    if !(f_src > 0.0 && c > 0.0 && v.is_finite()) {
        return None;
    }
    let beta = v / c;
    if beta.abs() >= 1.0 {
        return None;
    }
    let ratio = ((1.0 + beta) / (1.0 - beta)).sqrt();
    let observed = f_src * ratio;
    (observed.is_finite() && ratio.is_finite()).then_some((observed, ratio, beta))
}

fn local_emf_attenuation(
    source_power: f64,
    distance: f64,
    absorption_coeff: f64,
) -> Option<(f64, f64)> {
    if !(source_power > 0.0 && distance >= 0.0 && absorption_coeff.is_finite()) {
        return None;
    }
    let r = distance.max(1e-12);
    let alpha = absorption_coeff.max(0.0);
    let p_fs = source_power / (4.0 * std::f64::consts::PI * r * r);
    let p_rx = p_fs * (-alpha * r).exp();
    let atten_db = if p_rx > 0.0 {
        10.0 * (source_power / p_rx).log10()
    } else {
        f64::INFINITY
    };
    (p_rx.is_finite() && atten_db.is_finite()).then_some((p_rx, atten_db))
}

fn local_cfd_residual(velocity: &[f64]) -> Option<(f64, bool)> {
    let n = velocity.len();
    if n < 3 {
        return Some((f64::MAX, false));
    }
    let dx = 1.0 / n as f64;
    let nu = 1.5e-5_f64;
    let mut sumsq = 0.0f64;
    for i in 1..n - 1 {
        let u_x = (velocity[i + 1] - velocity[i - 1]) / (2.0 * dx);
        let u_xx = (velocity[i + 1] - 2.0 * velocity[i] + velocity[i - 1]) / (dx * dx);
        let residual = nu * u_xx - velocity[i] * u_x;
        sumsq += residual * residual;
    }
    let residual_norm = sumsq.sqrt();
    residual_norm
        .is_finite()
        .then_some((residual_norm, residual_norm < 1e-6))
}

fn default_heat_field() -> Vec<f64> {
    vec![0.0, 0.0, 1.0, 1.0, 0.0, 0.0]
}

fn default_wave_u0() -> Vec<f64> {
    (0..16)
        .map(|i| (std::f64::consts::PI * i as f64 / 15.0).sin())
        .collect()
}

fn default_quantum_potential() -> Vec<f64> {
    (0..16)
        .map(|i| {
            let x = i as f64 / 15.0 - 0.5;
            0.5 * x * x
        })
        .collect()
}

/// `Physics.doppler_shift`
pub(super) fn run_doppler_shift(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let source_frequency = numeric_attr(container.as_ref(), "data-source-frequency")
        .or_else(|| nums.first().copied())
        .unwrap_or(1.0e9);
    let relative_velocity = numeric_attr(container.as_ref(), "data-relative-velocity")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(0.1 * C_LIGHT);
    let c = numeric_attr(container.as_ref(), "data-c")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(C_LIGHT);
    let Some((observed, ratio, beta)) = local_doppler(source_frequency, relative_velocity, c)
    else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need positive frequency and |v| < c (attrs data-source-frequency / data-relative-velocity).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Physics.doppler_shift",
        format!("Doppler sketch: f_obs={observed:.6} ratio={ratio:.6} β={beta:.6}"),
        json!({
            "source_frequency": source_frequency,
            "relative_velocity": relative_velocity,
            "c": c,
        }),
    );
}

/// `Physics.emf_attenuation`
pub(super) fn run_emf_attenuation(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let source_power = numeric_attr(container.as_ref(), "data-source-power")
        .or_else(|| nums.first().copied())
        .unwrap_or(1.0);
    let frequency = numeric_attr(container.as_ref(), "data-frequency")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(1.0e9);
    let distance = numeric_attr(container.as_ref(), "data-distance")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(1.0);
    let absorption_coeff = numeric_attr(container.as_ref(), "data-absorption-coeff")
        .or_else(|| nums.get(3).copied())
        .unwrap_or(0.0);
    let Some((p_rx, atten_db)) = local_emf_attenuation(source_power, distance, absorption_coeff)
    else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need positive source_power and non-negative distance.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Physics.emf_attenuation",
        format!("EMF atten. sketch: P_rx={p_rx:.6e} atten_dB={atten_db:.4}"),
        json!({
            "source_power": source_power,
            "frequency": frequency,
            "distance": distance,
            "absorption_coeff": absorption_coeff,
        }),
    );
}

/// `Physics.harmonic_oscillator`
pub(super) fn run_harmonic_oscillator(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let mass = numeric_attr(container.as_ref(), "data-mass")
        .or_else(|| nums.first().copied())
        .unwrap_or(1.0);
    let k_spring = numeric_attr(container.as_ref(), "data-k-spring")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(1.0);
    let x0 = numeric_attr(container.as_ref(), "data-x0")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(1.0);
    let v0 = numeric_attr(container.as_ref(), "data-v0")
        .or_else(|| nums.get(3).copied())
        .unwrap_or(0.0);
    let total_time = numeric_attr(container.as_ref(), "data-total-time")
        .or_else(|| nums.get(4).copied())
        .unwrap_or(10.0);
    let samples = usize_attr(container.as_ref(), "data-samples")
        .or_else(|| nums.get(5).map(|v| *v as usize))
        .unwrap_or(64)
        .clamp(4, 256);
    if !(mass > 0.0 && k_spring > 0.0) {
        super::interactions::show_tool_status(
            document,
            label,
            "Need positive mass and spring constant.",
            "error",
        );
        return;
    }
    let period = 2.0 * std::f64::consts::PI * (mass / k_spring).sqrt();
    let energy = 0.5 * k_spring * x0 * x0 + 0.5 * mass * v0 * v0;
    invoke_dual(
        document,
        label,
        "Physics.harmonic_oscillator",
        format!("HO sketch: T≈{period:.4} E≈{energy:.4} (mass={mass}, k={k_spring})"),
        json!({
            "mass": mass,
            "k_spring": k_spring,
            "x0": x0,
            "v0": v0,
            "total_time": total_time,
            "samples": samples as u64,
        }),
    );
}

/// `Physics.pendulum`
pub(super) fn run_pendulum(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let length = numeric_attr(container.as_ref(), "data-length")
        .or_else(|| nums.first().copied())
        .unwrap_or(1.0);
    let g = numeric_attr(container.as_ref(), "data-g")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(9.81);
    let theta0 = numeric_attr(container.as_ref(), "data-theta0")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(std::f64::consts::FRAC_PI_6);
    let omega0 = numeric_attr(container.as_ref(), "data-omega0")
        .or_else(|| nums.get(3).copied())
        .unwrap_or(0.0);
    let total_time = numeric_attr(container.as_ref(), "data-total-time")
        .or_else(|| nums.get(4).copied())
        .unwrap_or(10.0);
    let samples = usize_attr(container.as_ref(), "data-samples")
        .or_else(|| nums.get(5).map(|v| *v as usize))
        .unwrap_or(64)
        .clamp(4, 256);
    if !(length > 0.0 && g > 0.0) {
        super::interactions::show_tool_status(
            document,
            label,
            "Need positive length and g.",
            "error",
        );
        return;
    }
    let small_angle_period = 2.0 * std::f64::consts::PI * (length / g).sqrt();
    invoke_dual(
        document,
        label,
        "Physics.pendulum",
        format!("Pendulum sketch: T_small≈{small_angle_period:.4} θ0={theta0:.4}"),
        json!({
            "length": length,
            "g": g,
            "theta0": theta0,
            "omega0": omega0,
            "total_time": total_time,
            "samples": samples as u64,
        }),
    );
}

/// `Physics.logistic_growth`
pub(super) fn run_logistic_growth(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let n0 = numeric_attr(container.as_ref(), "data-n0")
        .or_else(|| nums.first().copied())
        .unwrap_or(1.0);
    let growth_rate = numeric_attr(container.as_ref(), "data-growth-rate")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(0.1);
    let carrying_capacity = numeric_attr(container.as_ref(), "data-carrying-capacity")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(100.0);
    let total_time = numeric_attr(container.as_ref(), "data-total-time")
        .or_else(|| nums.get(3).copied())
        .unwrap_or(50.0);
    let samples = usize_attr(container.as_ref(), "data-samples")
        .or_else(|| nums.get(4).map(|v| *v as usize))
        .unwrap_or(50)
        .clamp(4, 256);
    if !(n0 > 0.0 && carrying_capacity > 0.0) {
        super::interactions::show_tool_status(
            document,
            label,
            "Need positive n0 and carrying capacity.",
            "error",
        );
        return;
    }
    let n_end = carrying_capacity
        / (1.0 + (carrying_capacity / n0 - 1.0) * (-growth_rate * total_time).exp());
    invoke_dual(
        document,
        label,
        "Physics.logistic_growth",
        format!("Logistic sketch: N({total_time})≈{n_end:.4} (K={carrying_capacity})"),
        json!({
            "n0": n0,
            "growth_rate": growth_rate,
            "carrying_capacity": carrying_capacity,
            "total_time": total_time,
            "samples": samples as u64,
        }),
    );
}

/// `Physics.cfd_step`
pub(super) fn run_cfd_step(document: &Document, label: &str) {
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let velocity = if nums.len() >= 3 {
        nums
    } else {
        vec![0.0, 0.1, 0.2, 0.1, 0.0]
    };
    let Some((residual_norm, converged)) = local_cfd_residual(&velocity) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need a velocity field of at least three finite numbers.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Physics.cfd_step",
        format!(
            "CFD residual sketch over {} nodes: ‖r‖={residual_norm:.6e} converged={converged}",
            velocity.len()
        ),
        json!({ "velocity": velocity }),
    );
}

/// `Physics.heat_diffusion_1d`
pub(super) fn run_heat_diffusion_1d(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let initial = if nums.len() >= 3 {
        nums
    } else {
        default_heat_field()
    };
    let alpha = numeric_attr(container.as_ref(), "data-alpha").unwrap_or(0.1);
    let dx = numeric_attr(container.as_ref(), "data-dx").unwrap_or(0.1);
    let total_time = numeric_attr(container.as_ref(), "data-total-time").unwrap_or(1.0);
    let samples = usize_attr(container.as_ref(), "data-samples")
        .unwrap_or(16)
        .clamp(4, 128);
    let mean = initial.iter().sum::<f64>() / initial.len() as f64;
    invoke_dual(
        document,
        label,
        "Physics.heat_diffusion_1d",
        format!(
            "Heat sketch: {} cells mean≈{mean:.4} α={alpha} (relaxes toward mean)",
            initial.len()
        ),
        json!({
            "initial": initial,
            "alpha": alpha,
            "dx": dx,
            "total_time": total_time,
            "samples": samples as u64,
        }),
    );
}

/// `Physics.wave_1d`
pub(super) fn run_wave_1d(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (u0, v0) = if nums.len() >= 8 {
        let half = nums.len() / 2;
        (nums[..half].to_vec(), nums[half..].to_vec())
    } else if nums.len() >= 4 {
        let v0 = vec![0.0; nums.len()];
        (nums, v0)
    } else {
        let u0 = default_wave_u0();
        let v0 = vec![0.0; u0.len()];
        (u0, v0)
    };
    let n = u0.len().min(v0.len());
    let u0 = u0[..n].to_vec();
    let v0 = v0[..n].to_vec();
    let c = numeric_attr(container.as_ref(), "data-c").unwrap_or(1.0);
    let dx = numeric_attr(container.as_ref(), "data-dx").unwrap_or(0.05);
    let total_time = numeric_attr(container.as_ref(), "data-total-time").unwrap_or(0.5);
    let samples = usize_attr(container.as_ref(), "data-samples")
        .unwrap_or(16)
        .clamp(4, 128);
    let energy_proxy: f64 = u0.iter().map(|x| x * x).sum::<f64>()
        + v0.iter().map(|x| x * x).sum::<f64>();
    invoke_dual(
        document,
        label,
        "Physics.wave_1d",
        format!("Wave sketch: {n} nodes energy≈{energy_proxy:.4} c={c}"),
        json!({
            "u0": u0,
            "v0": v0,
            "c": c,
            "dx": dx,
            "total_time": total_time,
            "samples": samples as u64,
        }),
    );
}

/// `Physics.advection_diffusion_1d`
pub(super) fn run_advection_diffusion_1d(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let initial = if nums.len() >= 4 {
        nums
    } else {
        default_heat_field()
    };
    let velocity = numeric_attr(container.as_ref(), "data-velocity").unwrap_or(1.0);
    let diffusion_coeff =
        numeric_attr(container.as_ref(), "data-diffusion-coeff").unwrap_or(0.01);
    let dx = numeric_attr(container.as_ref(), "data-dx").unwrap_or(0.1);
    let total_time = numeric_attr(container.as_ref(), "data-total-time").unwrap_or(1.0);
    let samples = usize_attr(container.as_ref(), "data-samples")
        .unwrap_or(16)
        .clamp(4, 128);
    let total = initial.iter().sum::<f64>();
    invoke_dual(
        document,
        label,
        "Physics.advection_diffusion_1d",
        format!(
            "Adv–diff sketch: {} cells total≈{total:.4} v={velocity} D={diffusion_coeff}",
            initial.len()
        ),
        json!({
            "initial": initial,
            "velocity": velocity,
            "diffusion_coeff": diffusion_coeff,
            "dx": dx,
            "total_time": total_time,
            "samples": samples as u64,
        }),
    );
}

/// `Physics.quantum_states_1d` — classical TISE eigenproblem (natural units).
pub(super) fn run_quantum_states_1d(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let potential = if nums.len() >= 4 {
        nums
    } else {
        default_quantum_potential()
    };
    let dx = numeric_attr(container.as_ref(), "data-dx").unwrap_or(0.1);
    let mass = numeric_attr(container.as_ref(), "data-mass").unwrap_or(1.0);
    let hbar = numeric_attr(container.as_ref(), "data-hbar").unwrap_or(1.0);
    let levels = usize_attr(container.as_ref(), "data-levels")
        .unwrap_or(3)
        .clamp(1, 8);
    let v_mean = potential.iter().sum::<f64>() / potential.len() as f64;
    invoke_dual(
        document,
        label,
        "Physics.quantum_states_1d",
        format!(
            "TISE sketch: {} grid pts ⟨V⟩≈{v_mean:.4} levels={levels} (ħ={hbar})",
            potential.len()
        ),
        json!({
            "potential": potential,
            "dx": dx,
            "mass": mass,
            "hbar": hbar,
            "levels": levels as u64,
        }),
    );
}

fn default_emf_sources() -> Vec<f64> {
    // One source at origin: [x,y,z,A,f,φ]
    vec![0.0, 0.0, 0.0, 1.0, 1.0, 0.0]
}

fn default_n_body_masses() -> Vec<f64> {
    vec![1.0, 1.0]
}

fn default_n_body_positions() -> Vec<f64> {
    vec![-1.0, 0.0, 1.0, 0.0]
}

fn default_n_body_velocities() -> Vec<f64> {
    vec![0.0, 0.5, 0.0, -0.5]
}

fn default_md_positions() -> Vec<f64> {
    vec![0.0, 0.0, 2.0, 0.0]
}

fn local_emf_amp_sketch(sources: &[f64], x: f64, y: f64, z: f64) -> Option<(usize, f64)> {
    if sources.len() < 6 || sources.len() % 6 != 0 {
        return None;
    }
    let n = sources.len() / 6;
    let mut amp_sum = 0.0f64;
    for i in 0..n {
        let b = i * 6;
        let dx = x - sources[b];
        let dy = y - sources[b + 1];
        let dz = z - sources[b + 2];
        let a = sources[b + 3];
        let r = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-12);
        amp_sum += a / r;
    }
    amp_sum.is_finite().then_some((n, amp_sum))
}

/// `Physics.n_body` — 2D Newtonian N-body (direct sum).
pub(super) fn run_n_body(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (masses, positions, velocities) = if nums.len() >= 10 {
        // Prefer attrs when present; else split flat: n masses then 2n pos then 2n vel.
        let n_guess = nums.len() / 5;
        let n = n_guess.max(2);
        if nums.len() >= n + 4 * n {
            let masses = nums[..n].to_vec();
            let positions = nums[n..n + 2 * n].to_vec();
            let velocities = nums[n + 2 * n..n + 4 * n].to_vec();
            (masses, positions, velocities)
        } else {
            (
                default_n_body_masses(),
                default_n_body_positions(),
                default_n_body_velocities(),
            )
        }
    } else {
        (
            default_n_body_masses(),
            default_n_body_positions(),
            default_n_body_velocities(),
        )
    };
    let g = numeric_attr(container.as_ref(), "data-g").unwrap_or(1.0);
    let softening = numeric_attr(container.as_ref(), "data-softening").unwrap_or(0.1);
    let total_time = numeric_attr(container.as_ref(), "data-total-time").unwrap_or(0.5);
    let samples = usize_attr(container.as_ref(), "data-samples")
        .unwrap_or(16)
        .clamp(4, 128);
    let n = masses.len();
    let ke: f64 = velocities
        .chunks(2)
        .zip(masses.iter())
        .map(|(v, m)| 0.5 * m * (v.get(0).copied().unwrap_or(0.0).powi(2) + v.get(1).copied().unwrap_or(0.0).powi(2)))
        .sum();
    invoke_dual(
        document,
        label,
        "Physics.n_body",
        format!("N-body sketch: {n} bodies KE≈{ke:.4} g={g} (soft={softening})"),
        json!({
            "masses": masses,
            "positions": positions,
            "velocities": velocities,
            "g": g,
            "softening": softening,
            "total_time": total_time,
            "samples": samples as u64,
        }),
    );
}

/// `Physics.molecular_dynamics` — 2D Lennard-Jones (velocity-Verlet).
pub(super) fn run_molecular_dynamics(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let (positions, velocities) = if nums.len() >= 8 {
        let half = nums.len() / 2;
        (nums[..half].to_vec(), nums[half..].to_vec())
    } else if nums.len() >= 4 {
        let v0 = vec![0.0; nums.len()];
        (nums, v0)
    } else {
        let p = default_md_positions();
        let v = vec![0.0; p.len()];
        (p, v)
    };
    let n = positions.len().min(velocities.len());
    let positions = positions[..n].to_vec();
    let velocities = velocities[..n].to_vec();
    let epsilon = numeric_attr(container.as_ref(), "data-epsilon").unwrap_or(1.0);
    let sigma = numeric_attr(container.as_ref(), "data-sigma").unwrap_or(1.0);
    let mass = numeric_attr(container.as_ref(), "data-mass").unwrap_or(1.0);
    let total_time = numeric_attr(container.as_ref(), "data-total-time").unwrap_or(0.1);
    let samples = usize_attr(container.as_ref(), "data-samples")
        .unwrap_or(8)
        .clamp(4, 128);
    let n_part = n / 2;
    let pair_r = if n >= 4 {
        let dx = positions[2] - positions[0];
        let dy = positions[3] - positions[1];
        (dx * dx + dy * dy).sqrt()
    } else {
        0.0
    };
    invoke_dual(
        document,
        label,
        "Physics.molecular_dynamics",
        format!("MD sketch: {n_part} particles r₁₂≈{pair_r:.4} ε={epsilon} σ={sigma}"),
        json!({
            "positions": positions,
            "velocities": velocities,
            "epsilon": epsilon,
            "sigma": sigma,
            "mass": mass,
            "total_time": total_time,
            "samples": samples as u64,
        }),
    );
}

/// `Physics.emf_interference` — superposition of N EMF sources at a 3D point.
pub(super) fn run_emf_interference(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let sources = if nums.len() >= 6 && nums.len() % 6 == 0 {
        nums
    } else {
        default_emf_sources()
    };
    let x = numeric_attr(container.as_ref(), "data-x").unwrap_or(1.0);
    let y = numeric_attr(container.as_ref(), "data-y").unwrap_or(0.0);
    let z = numeric_attr(container.as_ref(), "data-z").unwrap_or(0.0);
    let t = numeric_attr(container.as_ref(), "data-t").unwrap_or(0.0);
    let c = numeric_attr(container.as_ref(), "data-c").unwrap_or(1.0);
    let Some((n, amp_sum)) = local_emf_amp_sketch(&sources, x, y, z) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Need EMF sources as flat [x,y,z,A,f,φ,…] (multiple of 6).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Physics.emf_interference",
        format!("EMF interfere sketch: {n} sources Σ|A|/r≈{amp_sum:.4} at ({x},{y},{z})"),
        json!({
            "sources": sources,
            "x": x,
            "y": y,
            "z": z,
            "t": t,
            "c": c,
        }),
    );
}

/// `Physics.emf_field_grid_3d` — 4D physics grid (x×y×z×t).
pub(super) fn run_emf_field_grid_3d(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let sources = if nums.len() >= 6 && nums.len() % 6 == 0 {
        nums
    } else {
        default_emf_sources()
    };
    let bounds = [
        numeric_attr(container.as_ref(), "data-x-min").unwrap_or(-5.0),
        numeric_attr(container.as_ref(), "data-x-max").unwrap_or(5.0),
        numeric_attr(container.as_ref(), "data-y-min").unwrap_or(-5.0),
        numeric_attr(container.as_ref(), "data-y-max").unwrap_or(5.0),
        numeric_attr(container.as_ref(), "data-z-min").unwrap_or(-5.0),
        numeric_attr(container.as_ref(), "data-z-max").unwrap_or(5.0),
    ];
    let nx = usize_attr(container.as_ref(), "data-nx").unwrap_or(3).clamp(2, 8);
    let ny = usize_attr(container.as_ref(), "data-ny").unwrap_or(3).clamp(2, 8);
    let nz = usize_attr(container.as_ref(), "data-nz").unwrap_or(3).clamp(2, 8);
    let nt = usize_attr(container.as_ref(), "data-nt").unwrap_or(2).clamp(1, 4);
    let t_start = numeric_attr(container.as_ref(), "data-t-start").unwrap_or(0.0);
    let t_end = numeric_attr(container.as_ref(), "data-t-end").unwrap_or(1.0);
    let c = numeric_attr(container.as_ref(), "data-c").unwrap_or(1.0);
    let cells = nx * ny * nz * nt;
    let n_src = sources.len() / 6;
    invoke_dual(
        document,
        label,
        "Physics.emf_field_grid_3d",
        format!("EMF grid sketch: {nx}×{ny}×{nz}×{nt}={cells} cells, {n_src} sources"),
        json!({
            "sources": sources,
            "bounds": bounds,
            "nx": nx as u64,
            "ny": ny as u64,
            "nz": nz as u64,
            "nt": nt as u64,
            "t_start": t_start,
            "t_end": t_end,
            "c": c,
        }),
    );
}

/// `Physics.emf_sample_at_depth` — depth-aware sampling along a camera ray.
pub(super) fn run_emf_sample_at_depth(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let sources = if nums.len() >= 6 && nums.len() % 6 == 0 {
        nums
    } else {
        // Source ahead on +z for a useful default ray sample.
        vec![0.0, 0.0, 10.0, 1.0, 1.0, 0.0]
    };
    let camera = [
        numeric_attr(container.as_ref(), "data-cx").unwrap_or(0.0),
        numeric_attr(container.as_ref(), "data-cy").unwrap_or(0.0),
        numeric_attr(container.as_ref(), "data-cz").unwrap_or(0.0),
    ];
    let direction = [
        numeric_attr(container.as_ref(), "data-dx").unwrap_or(0.0),
        numeric_attr(container.as_ref(), "data-dy").unwrap_or(0.0),
        numeric_attr(container.as_ref(), "data-dz").unwrap_or(1.0),
    ];
    let depths = if let Some(d) = numeric_attr(container.as_ref(), "data-depth") {
        vec![d]
    } else {
        vec![1.0, 10.0, 100.0]
    };
    let t = numeric_attr(container.as_ref(), "data-t").unwrap_or(0.0);
    let c = numeric_attr(container.as_ref(), "data-c").unwrap_or(1.0);
    let n_src = sources.len() / 6;
    invoke_dual(
        document,
        label,
        "Physics.emf_sample_at_depth",
        format!(
            "EMF depth sketch: {} depths, {n_src} sources along ray",
            depths.len()
        ),
        json!({
            "sources": sources,
            "camera": camera,
            "direction": direction,
            "depths": depths,
            "t": t,
            "c": c,
        }),
    );
}

/// `Physics.field_sample` — ambient field value at a 3D position.
pub(super) fn run_field_sample(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let position = if nums.len() >= 3 {
        vec![nums[0], nums[1], nums[2]]
    } else {
        vec![0.0, 0.0, 0.0]
    };
    let field = container
        .as_ref()
        .and_then(|e| e.get_attribute("data-field"))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "atmosphere".into());
    let concentration = numeric_attr(container.as_ref(), "data-concentration").unwrap_or(55500.0);
    invoke_dual(
        document,
        label,
        "Physics.field_sample",
        format!(
            "Field sample sketch: {field} at [{:.3},{:.3},{:.3}]",
            position[0], position[1], position[2]
        ),
        json!({
            "field": field,
            "position": position,
            "concentration": concentration,
        }),
    );
}

/// `Physics.material_query` — faceted material signature traits.
pub(super) fn run_material_query(document: &Document, label: &str) {
    let container = selected_container(document);
    let material = container
        .as_ref()
        .and_then(|e| e.get_attribute("data-material"))
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            selected_source(document).and_then(|s| {
                let t = s.trim().to_lowercase();
                if t.is_empty() {
                    None
                } else {
                    Some(t.chars().take(64).collect())
                }
            })
        })
        .unwrap_or_else(|| "sugar_cube".into());
    invoke_dual(
        document,
        label,
        "Physics.material_query",
        format!("Material query sketch: id={material}"),
        json!({ "material": material }),
    );
}

/// `Physics.evaluate_interaction` — field laws on a continuant + ambient fields.
pub(super) fn run_evaluate_interaction(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let id = container
        .as_ref()
        .and_then(|e| e.get_attribute("data-continuant-id"))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "body_01".into());
    let material = container
        .as_ref()
        .and_then(|e| e.get_attribute("data-material"))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "sugar_cube".into());
    let mass_kg = numeric_attr(container.as_ref(), "data-mass-kg")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.01);
    let position = if nums.len() >= 4 {
        vec![nums[1], nums[2], nums[3]]
    } else if nums.len() >= 3 {
        vec![nums[0], nums[1], nums[2]]
    } else {
        vec![0.0, 0.0, 0.0]
    };
    let ambient_pressure_kpa = numeric_attr(container.as_ref(), "data-ambient-pressure-kpa");
    let mut args = json!({
        "id": id,
        "material": material,
        "mass_kg": mass_kg,
        "position": position,
    });
    if let Some(p) = ambient_pressure_kpa {
        args["ambient_pressure_kpa"] = json!(p);
    }
    let pressure_note = ambient_pressure_kpa
        .map(|p| format!(" P={p:.1} kPa"))
        .unwrap_or_default();
    invoke_dual(
        document,
        label,
        "Physics.evaluate_interaction",
        format!("Interaction sketch: {material} m={mass_kg:.4} kg{pressure_note}"),
        args,
    );
}
