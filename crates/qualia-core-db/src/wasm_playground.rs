//! JSON-string playground wrappers for docs/science-playground.html.
//!
//! These thin `#[wasm_bindgen]` exports accept `JSON.stringify(...)` payloads from
//! the browser demos and return JSON strings (not JsValue) so the HTML can
//! `JSON.parse(result)` uniformly.

#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
fn json_err(msg: impl AsRef<str>) -> JsValue {
    JsValue::from_str(msg.as_ref())
}

#[cfg(target_arch = "wasm32")]
fn to_json<T: Serialize>(val: &T) -> Result<String, JsValue> {
    serde_json::to_string(val).map_err(|e| json_err(e.to_string()))
}

#[cfg(target_arch = "wasm32")]
fn parse_json<T: for<'de> Deserialize<'de>>(s: &str) -> Result<T, JsValue> {
    serde_json::from_str(s).map_err(|e| json_err(e.to_string()))
}

#[cfg(target_arch = "wasm32")]
fn lcg_uniform(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
    (*seed as f64) / (u32::MAX as f64)
}

// ─── Geometric algebra ────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
struct GaInput {
    a: Vec<f32>,
    b: Vec<f32>,
    op: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Serialize)]
struct GaOutput {
    result: Vec<f32>,
    grades: [f64; 4],
    compute_ops: u32,
    op: String,
}

#[cfg(target_arch = "wasm32")]
fn mv_from_slice(c: &[f32]) -> crate::geometric_algebra::simd_kernel::Multivector {
    let mut coeffs = [0.0_f32; 8];
    for (i, v) in c.iter().take(8).enumerate() {
        coeffs[i] = *v;
    }
    crate::geometric_algebra::simd_kernel::Multivector {
        coeffs,
        grade_mask: 0,
    }
}

#[cfg(target_arch = "wasm32")]
fn grade_magnitudes(c: &[f32; 8]) -> [f64; 4] {
    let g0 = c[0].abs() as f64;
    let g1 = (c[1] * c[1] + c[2] * c[2] + c[3] * c[3]).sqrt() as f64;
    let g2 = (c[4] * c[4] + c[5] * c[5] + c[6] * c[6]).sqrt() as f64;
    let g3 = c[7].abs() as f64;
    [g0, g1, g2, g3]
}

#[cfg(target_arch = "wasm32")]
fn mv_norm(c: &[f32; 8]) -> f32 {
    c.iter().map(|x| x * x).sum::<f32>().sqrt()
}

#[cfg(target_arch = "wasm32")]
fn mv_exp_rotor(
    a: &crate::geometric_algebra::simd_kernel::Multivector,
) -> crate::geometric_algebra::simd_kernel::Multivector {
    use crate::geometric_algebra::simd_kernel::Multivector;
    let b = [a.coeffs[4], a.coeffs[5], a.coeffs[6]];
    let mag = (b[0] * b[0] + b[1] * b[1] + b[2] * b[2]).sqrt();
    if mag < f32::EPSILON {
        return Multivector::scalar(1.0);
    }
    let (sin_m, cos_m) = mag.sin_cos();
    let mut out = Multivector::zero();
    out.coeffs[0] = cos_m;
    out.coeffs[4] = sin_m * b[0] / mag;
    out.coeffs[5] = sin_m * b[1] / mag;
    out.coeffs[6] = sin_m * b[2] / mag;
    out
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn geometric_algebra_operation(input_json: &str) -> Result<String, JsValue> {
    use crate::geometric_algebra::simd_kernel::{geometric_product, outer_product, Multivector};

    let input: GaInput = parse_json(input_json)?;
    let a = mv_from_slice(&input.a);
    let b = mv_from_slice(&input.b);

    let out = match input.op.as_str() {
        "geo" => geometric_product(&a, &b),
        "inner" => {
            let ab = geometric_product(&a, &b);
            let ba = geometric_product(&b, &a);
            ab.add(&ba).div_scalar(2.0)
        }
        "outer" => outer_product(&a, &b),
        "reverse" => a.reverse(),
        "norm" => {
            let n = mv_norm(&a.coeffs);
            Multivector::scalar(n)
        }
        "exp" => mv_exp_rotor(&a),
        other => return Err(json_err(format!("unknown GA operation: {other}"))),
    };

    let grades = grade_magnitudes(&out.coeffs);
    to_json(&GaOutput {
        result: out.coeffs.to_vec(),
        grades,
        compute_ops: 64,
        op: input.op,
    })
}

// ─── Sequence alignment ───────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
struct AlignPlaygroundInput {
    query: String,
    target: String,
    mode: String,
    #[serde(default)]
    algo: Option<String>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn sequence_alignment(input_json: &str) -> Result<String, JsValue> {
    let input: AlignPlaygroundInput = parse_json(input_json)?;
    let result = if input.mode == "protein" {
        crate::domains::biological::bioinformatics::align_protein(
            input.query.as_bytes(),
            input.target.as_bytes(),
        )
    } else {
        crate::domains::biological::bioinformatics::align_nucleotide(
            input.query.as_bytes(),
            input.target.as_bytes(),
        )
    };

    #[derive(Serialize)]
    struct AlignOut {
        score: i32,
        identity_pct: f32,
        num_matches: usize,
        num_gaps: usize,
        aligned_query: String,
        aligned_target: String,
        mode: String,
        algorithm: String,
    }

    to_json(&AlignOut {
        score: result.score,
        identity_pct: result.identity_pct,
        num_matches: result.num_matches,
        num_gaps: result.num_gaps,
        aligned_query: String::from_utf8_lossy(&result.aligned_query).into_owned(),
        aligned_target: String::from_utf8_lossy(&result.aligned_target).into_owned(),
        mode: input.mode,
        algorithm: input.algo.unwrap_or_else(|| "smith-waterman".to_string()),
    })
}

// ─── Clinical risk ────────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
struct ClinicalRiskInput {
    calculator: String,
    params: serde_json::Value,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn clinical_risk(input_json: &str) -> Result<String, JsValue> {
    let input: ClinicalRiskInput = parse_json(input_json)?;
    let out =
        crate::clinical_playground::evaluate(&input.calculator, &input.params).map_err(json_err)?;
    to_json(&out)
}

// ─── Organic chemistry ────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
struct ChemPlaygroundInput {
    smiles: String,
    analysis: String,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn organic_chemistry(input_json: &str) -> Result<String, JsValue> {
    let input: ChemPlaygroundInput = parse_json(input_json)?;
    let mol = crate::domains::chemical::organic_chemistry::parse_smiles(&input.smiles);
    if !mol.is_valid {
        return Err(json_err(
            mol.error.unwrap_or_else(|| "Invalid SMILES".into()),
        ));
    }
    let desc = crate::domains::chemical::organic_chemistry::compute_descriptors(&mol);

    let out = match input.analysis.as_str() {
        "properties" => serde_json::json!({
            "analysis": "properties",
            "smiles": input.smiles,
            "molecular_weight": desc.molecular_weight,
            "formula": desc.formula,
            "heavy_atom_count": desc.heavy_atom_count,
            "hb_donors": desc.hb_donors,
            "hb_acceptors": desc.hb_acceptors,
            "rotatable_bonds": desc.rotatable_bonds,
            "aromatic_ring_count": desc.aromatic_ring_count,
            "ring_count": desc.ring_count,
            "logp_crippen": desc.logp_crippen,
            "tpsa_ertl": desc.tpsa_ertl,
            "chiral_centers": desc.chiral_centers,
            "fraction_csp3": desc.fraction_csp3,
        }),
        "lipinski" => {
            let lip = crate::domains::chemical::organic_chemistry::evaluate_lipinski(&desc);
            let veb = crate::domains::chemical::organic_chemistry::evaluate_veber(&desc);
            serde_json::json!({
                "analysis": "lipinski",
                "smiles": input.smiles,
                "lipinski_passes": lip.passes,
                "lipinski_violations": lip.violations,
                "veber_passes": veb.passes,
                "mw": desc.molecular_weight,
                "logp": desc.logp_crippen,
                "tpsa": desc.tpsa_ertl,
                "hbd": desc.hb_donors,
                "hba": desc.hb_acceptors,
                "rot_bonds": desc.rotatable_bonds,
            })
        }
        "druglikeness" => {
            let lip = crate::domains::chemical::organic_chemistry::evaluate_lipinski(&desc);
            let veb = crate::domains::chemical::organic_chemistry::evaluate_veber(&desc);
            let gho = crate::domains::chemical::organic_chemistry::evaluate_ghose(&desc);
            let ega = crate::domains::chemical::organic_chemistry::evaluate_egan(&desc);
            let passes = [lip.passes, veb.passes, gho.passes, ega.passes]
                .iter()
                .filter(|&&p| p)
                .count();
            serde_json::json!({
                "analysis": "druglikeness",
                "smiles": input.smiles,
                "druglikeness_score_pct": (passes as f64 / 4.0) * 100.0,
                "lipinski_passes": lip.passes,
                "veber_passes": veb.passes,
                "ghose_passes": gho.passes,
                "egan_passes": ega.passes,
                "mw": desc.molecular_weight,
                "logp": desc.logp_crippen,
                "tpsa": desc.tpsa_ertl,
            })
        }
        other => return Err(json_err(format!("unknown chemistry analysis: {other}"))),
    };

    to_json(&out)
}

// ─── ODE solver (RK4 presets) ─────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
struct OdePlaygroundInput {
    preset: String,
    h: f64,
    steps: usize,
}

#[cfg(target_arch = "wasm32")]
#[derive(Serialize)]
struct PhasePoint {
    x: f64,
    y: f64,
}

#[cfg(target_arch = "wasm32")]
#[derive(Serialize)]
struct OdePlaygroundOutput {
    preset: String,
    steps: usize,
    phase_space: Vec<PhasePoint>,
    time_points: Vec<f64>,
    x_values: Vec<f64>,
    y_values: Vec<f64>,
}

#[cfg(target_arch = "wasm32")]
fn rk4_step_2d(y: &mut [f64; 2], t: f64, h: f64, f: &dyn Fn(f64, &[f64; 2], &mut [f64; 2])) {
    let mut k1 = [0.0; 2];
    let mut k2 = [0.0; 2];
    let mut k3 = [0.0; 2];
    let mut k4 = [0.0; 2];
    let mut tmp = [0.0; 2];

    f(t, y, &mut k1);
    tmp[0] = y[0] + 0.5 * h * k1[0];
    tmp[1] = y[1] + 0.5 * h * k1[1];
    f(t + 0.5 * h, &tmp, &mut k2);
    tmp[0] = y[0] + 0.5 * h * k2[0];
    tmp[1] = y[1] + 0.5 * h * k2[1];
    f(t + 0.5 * h, &tmp, &mut k3);
    tmp[0] = y[0] + h * k3[0];
    tmp[1] = y[1] + h * k3[1];
    f(t + h, &tmp, &mut k4);

    y[0] += (h / 6.0) * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]);
    y[1] += (h / 6.0) * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn ode_solver(input_json: &str) -> Result<String, JsValue> {
    let input: OdePlaygroundInput = parse_json(input_json)?;
    if input.h <= 0.0 {
        return Err(json_err("step size h must be positive"));
    }
    let steps = input.steps.clamp(10, 10_000);

    let (y0, rhs): ([f64; 2], Box<dyn Fn(f64, &[f64; 2], &mut [f64; 2])>) =
        match input.preset.as_str() {
            "sho" => (
                [1.0, 0.0],
                Box::new(|_t, y, out| {
                    out[0] = y[1];
                    out[1] = -y[0];
                }),
            ),
            "lorenz" => (
                [1.0, 1.0],
                Box::new(|_t, y, out| {
                    let sigma = 10.0;
                    let rho = 28.0;
                    let z = 25.0;
                    let x = y[0];
                    let yy = y[1];
                    out[0] = sigma * (yy - x);
                    out[1] = x * (rho - z) - yy;
                }),
            ),
            "logistic" => (
                [0.1, 0.0],
                Box::new(|_t, y, out| {
                    let r = 4.0;
                    let k = 1.0;
                    let x = y[0];
                    let dx = r * x * (1.0 - x / k);
                    out[0] = dx;
                    out[1] = -0.5 * x + dx;
                }),
            ),
            "vanderpol" => (
                [2.0, 0.0],
                Box::new(|_t, y, out| {
                    let mu = 1.0;
                    out[0] = y[1];
                    out[1] = mu * (1.0 - y[0] * y[0]) * y[1] - y[0];
                }),
            ),
            "pendulum" => (
                [1.2, 0.0],
                Box::new(|_t, y, out| {
                    let g = 9.81;
                    let l = 1.0;
                    let b = 0.15;
                    out[0] = y[1];
                    out[1] = -(g / l) * y[0].sin() - b * y[1];
                }),
            ),
            "lotka" => (
                [10.0, 5.0],
                Box::new(|_t, y, out| {
                    let alpha = 1.1;
                    let beta = 0.4;
                    let delta = 0.1;
                    let gamma = 0.4;
                    let x = y[0];
                    let prey = y[1];
                    out[0] = alpha * x - beta * x * prey;
                    out[1] = delta * x * prey - gamma * prey;
                }),
            ),
            other => return Err(json_err(format!("unknown ODE preset: {other}"))),
        };

    let mut y = y0;
    let mut t = 0.0;
    let mut phase_space = Vec::with_capacity(steps + 1);
    let mut time_points = Vec::with_capacity(steps + 1);
    let mut x_values = Vec::with_capacity(steps + 1);
    let mut y_values = Vec::with_capacity(steps + 1);

    for _ in 0..=steps {
        phase_space.push(PhasePoint { x: y[0], y: y[1] });
        time_points.push(t);
        x_values.push(y[0]);
        y_values.push(y[1]);
        rk4_step_2d(&mut y, t, input.h, rhs.as_ref());
        t += input.h;
    }

    to_json(&OdePlaygroundOutput {
        preset: input.preset,
        steps,
        phase_space,
        time_points,
        x_values,
        y_values,
    })
}

// ─── Thermodynamics MCMC ──────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
struct ThermoPlaygroundInput {
    temp: f64,
    iterations: usize,
    system: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Serialize)]
struct ThermoPlaygroundOutput {
    system: String,
    temperature_k: f64,
    energy_levels: Vec<String>,
    probabilities: Vec<f64>,
    iterations: Vec<usize>,
    gibbs_trace: Vec<f64>,
    acceptance_rate: f64,
}

#[cfg(target_arch = "wasm32")]
fn energy_for_state(system: &str, state: i32, temp: f64) -> f64 {
    match system {
        "ideal" => 1.5 * (state as f64 + 1.0) * 8.617_333_262_145e-5 * temp,
        "van_der_waals" => {
            let v = 1.0 + 0.15 * state as f64;
            -0.4 / v + 0.05 / (v * v)
        }
        "isothermal" => 0.02 * (state as f64).powi(2),
        _ => 0.5 * (state as f64).powi(2),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn thermodynamics_mcmc(input_json: &str) -> Result<String, JsValue> {
    use crate::domains::physical::thermodynamics::ThermodynamicSampler;

    let input: ThermoPlaygroundInput = parse_json(input_json)?;
    let temp = input.temp.clamp(100.0, 2000.0);
    let iterations = input.iterations.clamp(100, 100_000);

    let mut sampler = ThermodynamicSampler::new(temp, 64);
    let mut seed = 0xC0FFEE_u64;
    let mut state = 0_i32;
    let mut accepted = 0_u32;

    let mut hist = [0_u32; 11];
    let mut gibbs_trace = Vec::with_capacity(200);
    let mut iter_labels = Vec::with_capacity(200);

    let initial_energy = energy_for_state(&input.system, state, temp);
    sampler.current_state.total_energy = initial_energy;

    for i in 0..iterations {
        let proposal = state + if lcg_uniform(&mut seed) < 0.5 { -1 } else { 1 };
        let clamped = proposal.clamp(-5, 5);
        let proposed_energy = energy_for_state(&input.system, clamped, temp);
        if sampler.metropolis_step(proposed_energy, lcg_uniform(&mut seed)) {
            state = clamped;
            accepted += 1;
        }
        let idx = (state + 5) as usize;
        hist[idx] = hist[idx].saturating_add(1);

        if i % (iterations / 200).max(1) == 0 {
            let entropy = -(hist.iter().filter(|&&c| c > 0).count() as f64).ln().abs();
            let gibbs =
                sampler.calculate_gibbs_free_energy(sampler.current_state.total_energy, entropy);
            gibbs_trace.push(gibbs);
            iter_labels.push(i);
        }
    }

    let total_samples: u32 = hist.iter().sum();
    let energy_levels: Vec<String> = (-5..=5).map(|s| format!("E{s}")).collect();
    let probabilities: Vec<f64> = hist
        .iter()
        .map(|&c| {
            if total_samples == 0 {
                0.0
            } else {
                c as f64 / total_samples as f64
            }
        })
        .collect();

    to_json(&ThermoPlaygroundOutput {
        system: input.system.clone(),
        temperature_k: temp,
        energy_levels,
        probabilities,
        iterations: iter_labels,
        gibbs_trace,
        acceptance_rate: if iterations == 0 {
            0.0
        } else {
            accepted as f64 / iterations as f64
        },
    })
}
