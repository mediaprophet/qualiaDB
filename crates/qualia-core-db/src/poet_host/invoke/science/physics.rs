//! Physics `capability.invoke` wrappers (real ODE/PDE/eigen solver paths).
//!
//! Every function here marshals Vibe `Value` args into the tested
//! `PhysicsSimulationLibrary` solvers and shapes the result back into a
//! `Value::Record`. No physics is re-derived — the numerics delegate to
//! `integrate_dopri5`, `integrate_symplectic`, and `symmetric_eigen`.

use super::super::args;
use crate::specialized_libs::physics_simulation::PhysicsSimulationLibrary;
use poet_vibe::{Diagnostic, Span, Value};

/// Convert `Vec<Vec<f64>>` snapshots into a Vipe `List<List<F64>>`.
fn f64_matrix(xs: &[Vec<f64>]) -> Value {
    Value::List(
        xs.iter()
            .map(|row| args::f64_list_value(row.iter().copied()))
            .collect(),
    )
}

/// `PhysicsAndODE.projectile` — 2D ballistic motion with optional drag.
pub fn projectile(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let v0 = args::rec_f64(args_v, "v0").unwrap_or(10.0);
    let angle = args::rec_f64(args_v, "angle_rad").unwrap_or(std::f64::consts::FRAC_PI_4);
    let g = args::rec_f64(args_v, "g").unwrap_or(9.81);
    let drag = args::rec_f64(args_v, "drag").unwrap_or(0.0);
    let samples = args::rec_u64(args_v, "samples").unwrap_or(64) as usize;
    let max_time = args::rec_f64(args_v, "max_time").unwrap_or(10.0);
    let mut lib = PhysicsSimulationLibrary::new();
    lib.initialize()
        .map_err(|e| args::bad(span, format!("physics init: {e:?}")))?;
    let r = lib
        .run_projectile_motion(v0, angle, g, drag, samples, max_time)
        .map_err(|e| args::bad(span, format!("projectile: {e:?}")))?;
    Ok(args::record([
        ("range", Value::F64(r.range)),
        ("max_height", Value::F64(r.max_height)),
        ("time_of_flight", Value::F64(r.time_of_flight)),
        ("landed", Value::Bool(r.landed)),
    ]))
}

/// `Physics.wave_1d` — 1D scalar wave equation `u_tt = c²·u_xx` (Dirichlet ends).
pub fn wave_1d(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let u0 = args::rec_f64_list(args_v, "u0").ok_or_else(|| {
        args::bad(
            span,
            "wave_1d needs { u0: [f64], v0: [f64], c, dx, total_time, samples }",
        )
    })?;
    let v0 = args::rec_f64_list(args_v, "v0").unwrap_or_else(|| vec![0.0; u0.len()]);
    let c = args::rec_f64(args_v, "c").unwrap_or(1.0);
    let dx = args::rec_f64(args_v, "dx").unwrap_or(0.01);
    let total_time = args::rec_f64(args_v, "total_time").unwrap_or(1.0);
    let samples = args::rec_u64(args_v, "samples").unwrap_or(32) as usize;
    let mut lib = PhysicsSimulationLibrary::new();
    lib.initialize()
        .map_err(|e| args::bad(span, format!("physics init: {e:?}")))?;
    let r = lib
        .run_wave_equation_1d(u0, v0, c, dx, total_time, samples)
        .map_err(|e| args::bad(span, format!("wave_1d: {e:?}")))?;
    Ok(args::record([
        ("energy_initial", Value::F64(r.energy_initial)),
        ("energy_final", Value::F64(r.energy_final)),
        ("times", args::f64_list_value(r.times.iter().copied())),
        ("snapshots", f64_matrix(&r.snapshots)),
        (
            "final_displacement",
            args::f64_list_value(r.final_displacement.iter().copied()),
        ),
        ("steps_accepted", Value::U64(r.steps_accepted as u64)),
        ("steps_rejected", Value::U64(r.steps_rejected as u64)),
    ]))
}

/// `Physics.heat_diffusion_1d` — `u_t = α·u_xx` (insulated/Neumann ends).
pub fn heat_diffusion_1d(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let initial = args::rec_f64_list(args_v, "initial").ok_or_else(|| {
        args::bad(
            span,
            "heat_diffusion_1d needs { initial: [f64], alpha, dx, total_time, samples }",
        )
    })?;
    let alpha = args::rec_f64(args_v, "alpha").unwrap_or(0.1);
    let dx = args::rec_f64(args_v, "dx").unwrap_or(0.01);
    let total_time = args::rec_f64(args_v, "total_time").unwrap_or(1.0);
    let samples = args::rec_u64(args_v, "samples").unwrap_or(32) as usize;
    let mut lib = PhysicsSimulationLibrary::new();
    lib.initialize()
        .map_err(|e| args::bad(span, format!("physics init: {e:?}")))?;
    let r = lib
        .run_heat_diffusion_1d(initial, alpha, dx, total_time, samples)
        .map_err(|e| args::bad(span, format!("heat_diffusion_1d: {e:?}")))?;
    Ok(args::record([
        ("initial_mean", Value::F64(r.initial_mean)),
        ("final_mean", Value::F64(r.final_mean)),
        (
            "max_deviation_from_mean",
            Value::F64(r.max_deviation_from_mean),
        ),
        ("times", args::f64_list_value(r.times.iter().copied())),
        ("snapshots", f64_matrix(&r.snapshots)),
        (
            "final_temperature",
            args::f64_list_value(r.final_temperature.iter().copied()),
        ),
        ("steps_accepted", Value::U64(r.steps_accepted as u64)),
        ("steps_rejected", Value::U64(r.steps_rejected as u64)),
    ]))
}

/// `Physics.advection_diffusion_1d` — `u_t + c·u_x = α·u_xx` (periodic grid).
pub fn advection_diffusion_1d(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let initial = args::rec_f64_list(args_v, "initial").ok_or_else(|| {
        args::bad(span, "advection_diffusion_1d needs { initial: [f64], velocity, diffusion_coeff, dx, total_time, samples }")
    })?;
    let velocity = args::rec_f64(args_v, "velocity").unwrap_or(1.0);
    let diffusion_coeff = args::rec_f64(args_v, "diffusion_coeff").unwrap_or(0.01);
    let dx = args::rec_f64(args_v, "dx").unwrap_or(0.01);
    let total_time = args::rec_f64(args_v, "total_time").unwrap_or(1.0);
    let samples = args::rec_u64(args_v, "samples").unwrap_or(32) as usize;
    let mut lib = PhysicsSimulationLibrary::new();
    lib.initialize()
        .map_err(|e| args::bad(span, format!("physics init: {e:?}")))?;
    let r = lib
        .run_advection_diffusion_1d(initial, velocity, diffusion_coeff, dx, total_time, samples)
        .map_err(|e| args::bad(span, format!("advection_diffusion_1d: {e:?}")))?;
    Ok(args::record([
        ("advection_velocity", Value::F64(r.advection_velocity)),
        ("diffusion_coeff", Value::F64(r.diffusion_coeff)),
        ("initial_total", Value::F64(r.initial_total)),
        ("final_total", Value::F64(r.final_total)),
        ("times", args::f64_list_value(r.times.iter().copied())),
        ("snapshots", f64_matrix(&r.snapshots)),
        (
            "final_field",
            args::f64_list_value(r.final_field.iter().copied()),
        ),
        ("steps_accepted", Value::U64(r.steps_accepted as u64)),
        ("steps_rejected", Value::U64(r.steps_rejected as u64)),
    ]))
}

/// `Physics.harmonic_oscillator` — spring–mass, symplectic Yoshida4 integrator.
pub fn harmonic_oscillator(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mass = args::rec_f64(args_v, "mass").unwrap_or(1.0);
    let k_spring = args::rec_f64(args_v, "k_spring").unwrap_or(1.0);
    let x0 = args::rec_f64(args_v, "x0").unwrap_or(1.0);
    let v0 = args::rec_f64(args_v, "v0").unwrap_or(0.0);
    let total_time = args::rec_f64(args_v, "total_time").unwrap_or(10.0);
    let samples = args::rec_u64(args_v, "samples").unwrap_or(100) as usize;
    let mut lib = PhysicsSimulationLibrary::new();
    lib.initialize()
        .map_err(|e| args::bad(span, format!("physics init: {e:?}")))?;
    let r = lib
        .run_harmonic_oscillator(mass, k_spring, x0, v0, total_time, samples)
        .map_err(|e| args::bad(span, format!("harmonic_oscillator: {e:?}")))?;
    Ok(args::record([
        ("analytic_period", Value::F64(r.analytic_period)),
        ("measured_period", Value::F64(r.measured_period)),
        ("energy_initial", Value::F64(r.energy_initial)),
        ("energy_final", Value::F64(r.energy_final)),
        ("max_energy_drift", Value::F64(r.max_energy_drift)),
        ("times", args::f64_list_value(r.times.iter().copied())),
        (
            "positions",
            args::f64_list_value(r.positions.iter().copied()),
        ),
        (
            "velocities",
            args::f64_list_value(r.velocities.iter().copied()),
        ),
    ]))
}

/// `Physics.pendulum` — nonlinear rigid-body pendulum `θ̈ = -(g/L)·sin θ`.
pub fn pendulum(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let length = args::rec_f64(args_v, "length").unwrap_or(1.0);
    let g = args::rec_f64(args_v, "g").unwrap_or(9.81);
    let theta0 = args::rec_f64(args_v, "theta0").unwrap_or(std::f64::consts::FRAC_PI_6);
    let omega0 = args::rec_f64(args_v, "omega0").unwrap_or(0.0);
    let total_time = args::rec_f64(args_v, "total_time").unwrap_or(10.0);
    let samples = args::rec_u64(args_v, "samples").unwrap_or(100) as usize;
    let mut lib = PhysicsSimulationLibrary::new();
    lib.initialize()
        .map_err(|e| args::bad(span, format!("physics init: {e:?}")))?;
    let r = lib
        .run_pendulum(length, g, theta0, omega0, total_time, samples)
        .map_err(|e| args::bad(span, format!("pendulum: {e:?}")))?;
    Ok(args::record([
        ("small_angle_period", Value::F64(r.small_angle_period)),
        ("measured_period", Value::F64(r.measured_period)),
        ("energy_initial", Value::F64(r.energy_initial)),
        ("energy_final", Value::F64(r.energy_final)),
        ("times", args::f64_list_value(r.times.iter().copied())),
        ("angles", args::f64_list_value(r.angles.iter().copied())),
        (
            "angular_velocities",
            args::f64_list_value(r.angular_velocities.iter().copied()),
        ),
        ("steps_accepted", Value::U64(r.steps_accepted as u64)),
        ("steps_rejected", Value::U64(r.steps_rejected as u64)),
    ]))
}

/// `Physics.n_body` — Newtonian N-body gravitation (2D, direct sum).
pub fn n_body(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let masses = args::rec_f64_list(args_v, "masses").ok_or_else(|| {
        args::bad(span, "n_body needs { masses: [f64], positions: [f64], velocities: [f64], g, softening, total_time, samples }")
    })?;
    let positions =
        args::rec_f64_list(args_v, "positions").unwrap_or_else(|| vec![0.0; masses.len() * 2]);
    let velocities =
        args::rec_f64_list(args_v, "velocities").unwrap_or_else(|| vec![0.0; masses.len() * 2]);
    let g = args::rec_f64(args_v, "g").unwrap_or(6.67430e-11);
    let softening = args::rec_f64(args_v, "softening").unwrap_or(0.1);
    let total_time = args::rec_f64(args_v, "total_time").unwrap_or(1.0);
    let samples = args::rec_u64(args_v, "samples").unwrap_or(32) as usize;
    let mut lib = PhysicsSimulationLibrary::new();
    lib.initialize()
        .map_err(|e| args::bad(span, format!("physics init: {e:?}")))?;
    let r = lib
        .run_nbody_gravitation(
            masses, positions, velocities, g, softening, total_time, samples,
        )
        .map_err(|e| args::bad(span, format!("n_body: {e:?}")))?;
    Ok(args::record([
        ("num_bodies", Value::U64(r.num_bodies as u64)),
        ("energy_initial", Value::F64(r.energy_initial)),
        ("energy_final", Value::F64(r.energy_final)),
        ("energy_drift_rel", Value::F64(r.energy_drift_rel)),
        (
            "angular_momentum_initial",
            Value::F64(r.angular_momentum_initial),
        ),
        (
            "angular_momentum_final",
            Value::F64(r.angular_momentum_final),
        ),
        ("times", args::f64_list_value(r.times.iter().copied())),
        ("position_snapshots", f64_matrix(&r.position_snapshots)),
        (
            "final_positions",
            args::f64_list_value(r.final_positions.iter().copied()),
        ),
        (
            "final_velocities",
            args::f64_list_value(r.final_velocities.iter().copied()),
        ),
        ("steps_accepted", Value::U64(r.steps_accepted as u64)),
        ("steps_rejected", Value::U64(r.steps_rejected as u64)),
    ]))
}

/// `Physics.molecular_dynamics` — 2D Lennard-Jones particles (velocity-Verlet).
pub fn molecular_dynamics(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let positions = args::rec_f64_list(args_v, "positions").ok_or_else(|| {
        args::bad(span, "molecular_dynamics needs { positions: [f64], velocities: [f64], epsilon, sigma, mass, total_time, samples }")
    })?;
    let velocities =
        args::rec_f64_list(args_v, "velocities").unwrap_or_else(|| vec![0.0; positions.len()]);
    let epsilon = args::rec_f64(args_v, "epsilon").unwrap_or(1.0);
    let sigma = args::rec_f64(args_v, "sigma").unwrap_or(1.0);
    let mass = args::rec_f64(args_v, "mass").unwrap_or(1.0);
    let total_time = args::rec_f64(args_v, "total_time").unwrap_or(1.0);
    let samples = args::rec_u64(args_v, "samples").unwrap_or(32) as usize;
    let mut lib = PhysicsSimulationLibrary::new();
    lib.initialize()
        .map_err(|e| args::bad(span, format!("physics init: {e:?}")))?;
    let r = lib
        .run_molecular_dynamics(
            positions, velocities, epsilon, sigma, mass, total_time, samples,
        )
        .map_err(|e| args::bad(span, format!("molecular_dynamics: {e:?}")))?;
    Ok(args::record([
        ("num_particles", Value::U64(r.num_particles as u64)),
        ("energy_initial", Value::F64(r.energy_initial)),
        ("energy_final", Value::F64(r.energy_final)),
        ("energy_drift_rel", Value::F64(r.energy_drift_rel)),
        ("temperature", Value::F64(r.temperature)),
        ("times", args::f64_list_value(r.times.iter().copied())),
        (
            "final_positions",
            args::f64_list_value(r.final_positions.iter().copied()),
        ),
        (
            "final_velocities",
            args::f64_list_value(r.final_velocities.iter().copied()),
        ),
        ("steps_accepted", Value::U64(r.steps_accepted as u64)),
        ("steps_rejected", Value::U64(r.steps_rejected as u64)),
    ]))
}

/// `Physics.cfd_step` — Burgers steady-state residual of a velocity field.
///
/// Mirrors `PhysicsSolver::solve_cfd_step`: the L2 norm of
/// `ν·u_xx − u·u_x` over interior nodes. The solver/mesh params in the
/// original are unused (the residual is a pure function of the field), so
/// this wrapper passes the velocity field directly.
pub fn cfd_step(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let velocity = args::rec_f64_list(args_v, "velocity")
        .ok_or_else(|| args::bad(span, "cfd_step needs { velocity: [f64] }"))?;
    let n = velocity.len();
    if n < 3 {
        return Ok(args::record([
            ("iterations", Value::U64(0)),
            ("residual_norm", Value::F64(f64::MAX)),
            ("converged", Value::Bool(false)),
        ]));
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
    Ok(args::record([
        ("iterations", Value::U64(1)),
        ("residual_norm", Value::F64(residual_norm)),
        ("converged", Value::Bool(residual_norm < 1e-6)),
    ]))
}

/// `Physics.quantum_states_1d` — time-independent Schrödinger eigenproblem.
///
/// **Classical simulation** — no QPU required. The TISE is discretised by
/// finite differences and the Hamiltonian is diagonalised by the classical
/// `symmetric_eigen` (Jacobi) eigensolver on CPU. This simulates quantum
/// mechanics; it does not use quantum hardware.
pub fn quantum_states_1d(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let potential = args::rec_f64_list(args_v, "potential").ok_or_else(|| {
        args::bad(
            span,
            "quantum_states_1d needs { potential: [f64], dx, mass, hbar, levels }",
        )
    })?;
    let dx = args::rec_f64(args_v, "dx").unwrap_or(0.01);
    let mass = args::rec_f64(args_v, "mass").unwrap_or(1.0);
    let hbar = args::rec_f64(args_v, "hbar").unwrap_or(1.054571817e-34);
    let levels = args::rec_u64(args_v, "levels").unwrap_or(3) as usize;
    let mut lib = PhysicsSimulationLibrary::new();
    lib.initialize()
        .map_err(|e| args::bad(span, format!("physics init: {e:?}")))?;
    let r = lib
        .run_quantum_stationary_states_1d(potential, dx, mass, hbar, levels)
        .map_err(|e| args::bad(span, format!("quantum_states_1d: {e:?}")))?;
    Ok(args::record([
        (
            "eigenvalues",
            args::f64_list_value(r.eigenvalues.iter().copied()),
        ),
        ("num_grid_points", Value::U64(r.num_grid_points as u64)),
        ("dx", Value::F64(r.dx)),
    ]))
}

/// `Physics.logistic_growth` — population dynamics `dN/dt = r·N·(1 − N/K)`.
pub fn logistic_growth(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n0 = args::rec_f64(args_v, "n0").unwrap_or(1.0);
    let growth_rate = args::rec_f64(args_v, "growth_rate").unwrap_or(0.1);
    let carrying_capacity = args::rec_f64(args_v, "carrying_capacity").unwrap_or(100.0);
    let total_time = args::rec_f64(args_v, "total_time").unwrap_or(50.0);
    let samples = args::rec_u64(args_v, "samples").unwrap_or(50) as usize;
    let mut lib = PhysicsSimulationLibrary::new();
    lib.initialize()
        .map_err(|e| args::bad(span, format!("physics init: {e:?}")))?;
    let r = lib
        .run_logistic_growth(n0, growth_rate, carrying_capacity, total_time, samples)
        .map_err(|e| args::bad(span, format!("logistic_growth: {e:?}")))?;
    Ok(args::record([
        ("carrying_capacity", Value::F64(r.carrying_capacity)),
        ("growth_rate", Value::F64(r.growth_rate)),
        ("times", args::f64_list_value(r.times.iter().copied())),
        (
            "population",
            args::f64_list_value(r.population.iter().copied()),
        ),
        ("steps_accepted", Value::U64(r.steps_accepted as u64)),
        ("steps_rejected", Value::U64(r.steps_rejected as u64)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use poet_vibe::Value;
    use std::collections::BTreeMap;

    fn rec(pairs: &[(&str, Value)]) -> Value {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert((*k).into(), v.clone());
        }
        Value::Record(m)
    }

    #[test]
    fn wave_1d_energy_conservation() {
        let n = 50;
        let u0: Vec<f64> = (0..n)
            .map(|i| (std::f64::consts::PI * i as f64 / (n - 1) as f64).sin())
            .collect();
        let v0 = vec![0.0; n];
        let args = rec(&[
            (
                "u0",
                Value::List(u0.iter().map(|x| Value::F64(*x)).collect()),
            ),
            (
                "v0",
                Value::List(v0.iter().map(|x| Value::F64(*x)).collect()),
            ),
            ("c", Value::F64(1.0)),
            ("dx", Value::F64(0.02)),
            ("total_time", Value::F64(0.5)),
            ("samples", Value::U64(10)),
        ]);
        let r = wave_1d(&args, poet_vibe::Span::new(0, 0)).unwrap();
        match r {
            Value::Record(m) => {
                let ei = m.get("energy_initial").and_then(args::as_f64).unwrap();
                let ef = m.get("energy_final").and_then(args::as_f64).unwrap();
                assert!(ei > 0.0, "wave should have positive energy");
                assert!(
                    (ef - ei).abs() / ei < 0.1,
                    "energy should be roughly conserved: ei={ei} ef={ef}"
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn heat_diffusion_relaxes_to_mean() {
        let initial = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0];
        let args = rec(&[
            (
                "initial",
                Value::List(initial.iter().map(|x| Value::F64(*x)).collect()),
            ),
            ("alpha", Value::F64(0.1)),
            ("dx", Value::F64(0.1)),
            ("total_time", Value::F64(5.0)),
            ("samples", Value::U64(10)),
        ]);
        let r = heat_diffusion_1d(&args, poet_vibe::Span::new(0, 0)).unwrap();
        match r {
            Value::Record(m) => {
                let dev = m
                    .get("max_deviation_from_mean")
                    .and_then(args::as_f64)
                    .unwrap();
                assert!(dev < 0.5, "heat should diffuse: max_dev={dev}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn harmonic_oscillator_period_matches_analytic() {
        let args = rec(&[
            ("mass", Value::F64(1.0)),
            ("k_spring", Value::F64(1.0)),
            ("x0", Value::F64(1.0)),
            ("v0", Value::F64(0.0)),
            ("total_time", Value::F64(20.0)),
            ("samples", Value::U64(200)),
        ]);
        let r = harmonic_oscillator(&args, poet_vibe::Span::new(0, 0)).unwrap();
        match r {
            Value::Record(m) => {
                let analytic = m.get("analytic_period").and_then(args::as_f64).unwrap();
                let measured = m.get("measured_period").and_then(args::as_f64).unwrap();
                assert!(analytic > 0.0);
                assert!(
                    (measured - analytic).abs() / analytic < 0.05,
                    "period mismatch: analytic={analytic} measured={measured}"
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn pendulum_small_angle_period() {
        let args = rec(&[
            ("length", Value::F64(1.0)),
            ("g", Value::F64(9.81)),
            ("theta0", Value::F64(0.01)),
            ("total_time", Value::F64(10.0)),
            ("samples", Value::U64(100)),
        ]);
        let r = pendulum(&args, poet_vibe::Span::new(0, 0)).unwrap();
        match r {
            Value::Record(m) => {
                let small = m.get("small_angle_period").and_then(args::as_f64).unwrap();
                let measured = m.get("measured_period").and_then(args::as_f64).unwrap();
                let expected = 2.0 * std::f64::consts::PI * (1.0_f64 / 9.81).sqrt();
                assert!((small - expected).abs() < 1e-10);
                assert!(
                    (measured - expected).abs() / expected < 0.05,
                    "pendulum period: small={small} measured={measured}"
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn n_body_two_body_energy_conserved() {
        let masses = vec![1.0, 1.0];
        let positions = vec![-1.0, 0.0, 1.0, 0.0];
        let velocities = vec![0.0, 0.5, 0.0, -0.5];
        let args = rec(&[
            (
                "masses",
                Value::List(masses.iter().map(|x| Value::F64(*x)).collect()),
            ),
            (
                "positions",
                Value::List(positions.iter().map(|x| Value::F64(*x)).collect()),
            ),
            (
                "velocities",
                Value::List(velocities.iter().map(|x| Value::F64(*x)).collect()),
            ),
            ("g", Value::F64(1.0)),
            ("softening", Value::F64(0.1)),
            ("total_time", Value::F64(0.5)),
            ("samples", Value::U64(10)),
        ]);
        let r = n_body(&args, poet_vibe::Span::new(0, 0)).unwrap();
        match r {
            Value::Record(m) => {
                let drift = m.get("energy_drift_rel").and_then(args::as_f64).unwrap();
                assert!(drift < 0.2, "energy drift should be bounded: drift={drift}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn molecular_dynamics_runs() {
        let positions = vec![0.0, 0.0, 2.0, 0.0];
        let velocities = vec![0.0, 0.0, 0.0, 0.0];
        let args = rec(&[
            (
                "positions",
                Value::List(positions.iter().map(|x| Value::F64(*x)).collect()),
            ),
            (
                "velocities",
                Value::List(velocities.iter().map(|x| Value::F64(*x)).collect()),
            ),
            ("epsilon", Value::F64(1.0)),
            ("sigma", Value::F64(1.0)),
            ("mass", Value::F64(1.0)),
            ("total_time", Value::F64(0.1)),
            ("samples", Value::U64(5)),
        ]);
        let r = molecular_dynamics(&args, poet_vibe::Span::new(0, 0)).unwrap();
        match r {
            Value::Record(m) => {
                let n = m.get("num_particles").and_then(args::as_u64).unwrap();
                assert_eq!(n, 2);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn cfd_step_returns_residual() {
        let velocity: Vec<f64> = (0..50)
            .map(|i| (std::f64::consts::PI * i as f64 / 49.0).sin())
            .collect();
        let args = rec(&[(
            "velocity",
            Value::List(velocity.iter().map(|x| Value::F64(*x)).collect()),
        )]);
        let r = cfd_step(&args, poet_vibe::Span::new(0, 0)).unwrap();
        match r {
            Value::Record(m) => {
                let residual = m.get("residual_norm").and_then(args::as_f64).unwrap();
                assert!(residual >= 0.0, "residual must be non-negative");
                assert!(residual.is_finite(), "residual must be finite");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn quantum_states_returns_eigenvalues() {
        // Infinite square well: V=0 inside, high walls at edges.
        let n = 40;
        let potential: Vec<f64> = (0..n)
            .map(|i| if i == 0 || i == n - 1 { 1e6 } else { 0.0 })
            .collect();
        let args = rec(&[
            (
                "potential",
                Value::List(potential.iter().map(|x| Value::F64(*x)).collect()),
            ),
            ("dx", Value::F64(0.025)),
            ("mass", Value::F64(1.0)),
            ("hbar", Value::F64(1.0)),
            ("levels", Value::U64(3)),
        ]);
        let r = quantum_states_1d(&args, poet_vibe::Span::new(0, 0)).unwrap();
        match r {
            Value::Record(m) => {
                let eigs = m.get("eigenvalues");
                assert!(eigs.is_some(), "should return eigenvalues");
                if let Some(Value::List(xs)) = eigs {
                    assert_eq!(xs.len(), 3, "should return 3 levels");
                    // Eigenvalues should be ascending.
                    let vals: Vec<f64> = xs.iter().filter_map(args::as_f64).collect();
                    assert!(vals[1] > vals[0], "eigenvalues should be ascending");
                }
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn logistic_growth_approaches_carrying_capacity() {
        let args = rec(&[
            ("n0", Value::F64(1.0)),
            ("growth_rate", Value::F64(0.5)),
            ("carrying_capacity", Value::F64(100.0)),
            ("total_time", Value::F64(20.0)),
            ("samples", Value::U64(50)),
        ]);
        let r = logistic_growth(&args, poet_vibe::Span::new(0, 0)).unwrap();
        match r {
            Value::Record(m) => {
                if let Some(Value::List(pop)) = m.get("population") {
                    let final_n = pop.last().and_then(args::as_f64).unwrap();
                    assert!(
                        final_n > 50.0,
                        "population should approach K=100: final={final_n}"
                    );
                    assert!(
                        final_n <= 100.0,
                        "population should not exceed K: final={final_n}"
                    );
                }
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn advection_diffusion_total_conserved() {
        let initial = vec![0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        let args = rec(&[
            (
                "initial",
                Value::List(initial.iter().map(|x| Value::F64(*x)).collect()),
            ),
            ("velocity", Value::F64(0.5)),
            ("diffusion_coeff", Value::F64(0.01)),
            ("dx", Value::F64(0.1)),
            ("total_time", Value::F64(0.5)),
            ("samples", Value::U64(10)),
        ]);
        let r = advection_diffusion_1d(&args, poet_vibe::Span::new(0, 0)).unwrap();
        match r {
            Value::Record(m) => {
                let i0 = m.get("initial_total").and_then(args::as_f64).unwrap();
                let f0 = m.get("final_total").and_then(args::as_f64).unwrap();
                assert!(
                    (f0 - i0).abs() / i0.abs().max(1e-10) < 0.05,
                    "total should be conserved: i0={i0} f0={f0}"
                );
            }
            other => panic!("{other:?}"),
        }
    }
}
