//! Bounded adapter for the POET Physics Simulator and standalone ODE panel.

use super::super::args;
use crate::domains::physical::thermodynamics::{
    array_mppt_power, heat_loss_rate, phase_change_energy, thermal_efficiency, LithiumPack,
    SolarPanel, ThermodynamicSampler,
};
use crate::{q_hash, quantum_dft::ElectronDensity, NQuin};
use vibe::{Diagnostic, Span, Value};

const MAX_STEPS: usize = 4_096;
const MAX_FEATURES: usize = 64;
const MAX_ELECTRONS: usize = 32;
const MAX_DFT_RESOLUTION: usize = 32;
const MAX_PANELS: usize = 64;

fn finite(args_v: &Value, key: &str, span: Span) -> Result<f64, Diagnostic> {
    let value = args::rec_f64(args_v, key)
        .ok_or_else(|| args::bad(span, format!("Physics.compute needs finite `{key}`")))?;
    if !value.is_finite() {
        return Err(args::bad(span, format!("`{key}` must be finite")));
    }
    Ok(value)
}

fn positive(args_v: &Value, key: &str, span: Span) -> Result<f64, Diagnostic> {
    let value = finite(args_v, key, span)?;
    if value <= 0.0 {
        return Err(args::bad(
            span,
            format!("`{key}` must be greater than zero"),
        ));
    }
    Ok(value)
}

fn bounded_count(
    args_v: &Value,
    key: &str,
    default: usize,
    maximum: usize,
    span: Span,
) -> Result<usize, Diagnostic> {
    let value = args::rec_u64(args_v, key).unwrap_or(default as u64) as usize;
    if value == 0 || value > maximum {
        return Err(args::bad(
            span,
            format!("`{key}` must be between 1 and {maximum}"),
        ));
    }
    Ok(value)
}

fn lcg_uniform(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*seed >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
}

fn metropolis(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let temperature = positive(args_v, "temperature", span)?;
    let particles = bounded_count(args_v, "ensemble_size", 1_000, 1_000_000, span)?;
    let steps = bounded_count(args_v, "steps", 1_000, MAX_STEPS, span)?;
    let proposal_scale = match args::rec_f64(args_v, "proposal_scale") {
        Some(value) if value.is_finite() && value > 0.0 => value,
        Some(_) => {
            return Err(args::bad(
                span,
                "`proposal_scale` must be finite and greater than zero",
            ))
        }
        None => 0.025,
    };
    let mut seed = args::rec_u64(args_v, "seed").unwrap_or(42);
    let mut sampler = ThermodynamicSampler::new(temperature, particles);
    let mut accepted = 0usize;
    let mut energy_sum = 0.0;
    for _ in 0..steps {
        let proposal = sampler.current_state.total_energy
            + (lcg_uniform(&mut seed) * 2.0 - 1.0) * proposal_scale;
        if sampler.metropolis_step(proposal, lcg_uniform(&mut seed)) {
            accepted += 1;
        }
        energy_sum += sampler.current_state.total_energy;
    }
    Ok(args::record([
        ("steps", Value::U64(steps as u64)),
        ("accepted", Value::U64(accepted as u64)),
        (
            "acceptance_rate",
            Value::F64(accepted as f64 / steps as f64),
        ),
        (
            "final_energy_ev",
            Value::F64(sampler.current_state.total_energy),
        ),
        ("mean_energy_ev", Value::F64(energy_sum / steps as f64)),
        ("seed", Value::U64(seed)),
    ]))
}

fn ode_solver(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let dt = positive(args_v, "dt", span)?;
    let steps = bounded_count(args_v, "steps", 1_000, MAX_STEPS, span)?;
    let k1 = args::rec_f64(args_v, "k1").unwrap_or(0.5);
    let k2 = args::rec_f64(args_v, "k2").unwrap_or(0.3);
    let coupling = args::rec_f64(args_v, "coupling").unwrap_or(1.0);
    let mut y1 = finite(args_v, "y1", span)?;
    let mut y2 = finite(args_v, "y2", span)?;
    let deriv = |a: f64, b: f64| (-k1 * a, coupling * a - k2 * b);
    for _ in 0..steps {
        let (a1, b1) = deriv(y1, y2);
        let (a2, b2) = deriv(y1 + 0.5 * dt * a1, y2 + 0.5 * dt * b1);
        let (a3, b3) = deriv(y1 + 0.5 * dt * a2, y2 + 0.5 * dt * b2);
        let (a4, b4) = deriv(y1 + dt * a3, y2 + dt * b3);
        y1 += dt * (a1 + 2.0 * a2 + 2.0 * a3 + a4) / 6.0;
        y2 += dt * (b1 + 2.0 * b2 + 2.0 * b3 + b4) / 6.0;
        if !y1.is_finite() || !y2.is_finite() {
            return Err(args::bad(
                span,
                "ODE integration diverged to a non-finite state",
            ));
        }
    }
    Ok(args::record([
        ("method", Value::String("RK4".into())),
        ("steps", Value::U64(steps as u64)),
        ("time", Value::F64(dt * steps as f64)),
        ("y1", Value::F64(y1)),
        ("y2", Value::F64(y2)),
    ]))
}

fn dft(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let electrons = bounded_count(args_v, "electron_count", 2, MAX_ELECTRONS, span)?;
    let resolution = bounded_count(args_v, "resolution", 16, MAX_DFT_RESOLUTION, span)?;
    let mut quins = [NQuin {
        subject: 0,
        predicate: 0,
        object: 0,
        context: 0,
        metadata: 0,
        parity: 0,
    }; MAX_ELECTRONS];
    for (index, quin) in quins[..electrons].iter_mut().enumerate() {
        quin.subject = index as u64 + 1;
        quin.predicate = q_hash("HAS_ELECTRON");
        quin.object = index as u64;
        quin.parity = quin.subject ^ quin.predicate ^ quin.object;
    }
    let mut density = ElectronDensity::new(resolution);
    let energy_ev = density.calculate_ground_state_energy(&quins[..electrons]);
    Ok(args::record([
        ("method", Value::String("Thomas-Fermi LDA".into())),
        ("electron_count", Value::U64(electrons as u64)),
        ("grid_resolution", Value::U64(resolution as u64)),
        ("ground_state_energy_ev", Value::F64(energy_ev)),
    ]))
}

fn pinn(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let molecule = args::rec_f64_list(args_v, "molecule_features")
        .ok_or_else(|| args::bad(span, "PINN needs `molecule_features=[...]`"))?;
    let receptor = args::rec_f64_list(args_v, "receptor_features")
        .ok_or_else(|| args::bad(span, "PINN needs `receptor_features=[...]`"))?;
    if molecule.is_empty()
        || molecule.len() != receptor.len()
        || molecule.len() > MAX_FEATURES
        || molecule.iter().chain(&receptor).any(|v| !v.is_finite())
    {
        return Err(args::bad(
            span,
            "PINN feature lists must be finite, equal-length, and contain 1..64 values",
        ));
    }
    // Bounded embedded-network inference: tanh hidden activations plus a
    // symmetry residual that penalises disagreement between paired features.
    let mut hidden = [0.0f64; 8];
    let mut residual = 0.0;
    for (index, (&m, &r)) in molecule.iter().zip(&receptor).enumerate() {
        residual += (m - r) * (m - r);
        for (unit, slot) in hidden.iter_mut().enumerate() {
            let weight = (((index + 1) * (unit + 3)) as f64 * 0.173).sin();
            *slot += weight * m + (1.0 - weight.abs()) * r;
        }
    }
    let scale = molecule.len() as f64;
    let network = hidden
        .iter()
        .enumerate()
        .map(|(unit, value)| (value / scale).tanh() * (unit as f64 + 1.0) / 8.0)
        .sum::<f64>();
    let physics_residual = residual / scale;
    let affinity = -5.0 - network.abs() - physics_residual.sqrt();
    Ok(args::record([
        ("model", Value::String("bounded-pinn-8x1".into())),
        ("feature_count", Value::U64(molecule.len() as u64)),
        ("physics_residual", Value::F64(physics_residual)),
        ("binding_affinity_kcal_mol", Value::F64(affinity)),
    ]))
}

fn lithium_pack(args_v: &Value, span: Span) -> Result<(LithiumPack, f64), Diagnostic> {
    let pack = LithiumPack {
        cells_series: bounded_count(args_v, "cells_series", 4, 1_024, span)? as u32,
        cells_parallel: bounded_count(args_v, "cells_parallel", 2, 1_024, span)? as u32,
        cell_internal_resistance_ohm: positive(args_v, "cell_resistance", span)?,
        cell_capacity_ah: positive(args_v, "cell_capacity_ah", span)?,
    };
    let soc = finite(args_v, "soc", span)?;
    if !(0.0..=1.0).contains(&soc) {
        return Err(args::bad(span, "`soc` must be between 0 and 1"));
    }
    Ok((pack, soc))
}

fn solar_panel(args_v: &Value, span: Span) -> Result<SolarPanel, Diagnostic> {
    let fill_factor = finite(args_v, "fill_factor", span)?;
    if !(0.05..=0.95).contains(&fill_factor) {
        return Err(args::bad(
            span,
            "`fill_factor` must be between 0.05 and 0.95",
        ));
    }
    Ok(SolarPanel {
        short_circuit_current_a: positive(args_v, "short_circuit_current", span)?,
        open_circuit_voltage_v: positive(args_v, "open_circuit_voltage", span)?,
        fill_factor,
    })
}

/// `PhysicsWorkbench.compute` — typed dispatch for every Physics Simulator mode.
pub fn compute(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let operation = args::rec_str(args_v, "operation")
        .ok_or_else(|| args::bad(span, "Physics.compute needs `operation`"))?;
    match operation {
        "metropolis" => metropolis(args_v, span),
        "ode_solver" => ode_solver(args_v, span),
        "dft" => dft(args_v, span),
        "pinn" => pinn(args_v, span),
        "gibbs" => {
            let temperature = positive(args_v, "temperature", span)?;
            let sampler = ThermodynamicSampler::new(temperature, 1);
            Ok(Value::F64(sampler.calculate_gibbs_free_energy(
                finite(args_v, "enthalpy", span)?,
                finite(args_v, "entropy", span)?,
            )))
        }
        "cell_ocv" | "pack_ocv" | "terminal_voltage" | "deliverable_power" => {
            let (pack, soc) = lithium_pack(args_v, span)?;
            let value = match operation {
                "cell_ocv" => pack.cell_ocv(soc),
                "pack_ocv" => pack.pack_ocv(soc),
                "terminal_voltage" => {
                    pack.terminal_voltage(soc, finite(args_v, "load_current", span)?)
                }
                _ => pack.deliverable_power(soc, finite(args_v, "load_current", span)?),
            };
            Ok(args::record([
                ("value", Value::F64(value)),
                ("pack_resistance_ohm", Value::F64(pack.pack_resistance())),
                ("pack_capacity_ah", Value::F64(pack.pack_capacity_ah())),
            ]))
        }
        "max_power_point" | "array_mppt" => {
            let panel = solar_panel(args_v, span)?;
            let scan_steps = bounded_count(args_v, "scan_steps", 256, 4_096, span)? as u32;
            let (voltage, current, power) = panel.max_power_point(scan_steps);
            if operation == "max_power_point" {
                Ok(args::record([
                    ("voltage_v", Value::F64(voltage)),
                    ("current_a", Value::F64(current)),
                    ("power_w", Value::F64(power)),
                ]))
            } else {
                let count = bounded_count(args_v, "panel_count", 1, MAX_PANELS, span)?;
                let panels = [panel; MAX_PANELS];
                Ok(Value::F64(array_mppt_power(&panels[..count], scan_steps)))
            }
        }
        "heat_loss" => Ok(Value::F64(heat_loss_rate(
            finite(args_v, "u_value", span)?,
            positive(args_v, "area", span)?,
            finite(args_v, "delta_t", span)?,
        ))),
        "phase_change" => Ok(Value::F64(phase_change_energy(
            positive(args_v, "mass", span)?,
            positive(args_v, "latent_heat", span)?,
        ))),
        "thermal_efficiency" => Ok(Value::F64(thermal_efficiency(
            finite(args_v, "useful_power", span)?,
            finite(args_v, "u_value", span)?,
            positive(args_v, "area", span)?,
            finite(args_v, "delta_t", span)?,
        ))),
        _ => Err(args::bad(
            span,
            format!("unknown physics operation `{operation}`"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rk4_workbench_matches_decay_direction() {
        let input = args::record([
            ("operation", Value::String("ode_solver".into())),
            ("dt", Value::F64(0.01)),
            ("steps", Value::U64(100)),
            ("y1", Value::F64(1.0)),
            ("y2", Value::F64(0.0)),
        ]);
        let Value::Record(result) = compute(&input, Span::new(0, 0)).unwrap() else {
            panic!("expected record")
        };
        assert!(args::as_f64(result.get("y1").unwrap()).unwrap() < 1.0);
        assert!(args::as_f64(result.get("y2").unwrap()).unwrap() > 0.0);
    }
}
