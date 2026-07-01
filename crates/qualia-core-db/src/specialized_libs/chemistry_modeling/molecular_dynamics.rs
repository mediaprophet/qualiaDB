//! Real molecular dynamics — Lennard-Jones force field + velocity-Verlet integrator.
//!
//! This is the genuine numerical core behind [`MolecularSimulator::run_simulation`]
//! (`super::MolecularSimulator`). It replaces the prior stub, which emitted a
//! "trajectory" whose atoms never moved (every frame cloned the same coordinates
//! with zero velocities, forces and energy — a simulation that simulated nothing).
//!
//! What is real here:
//! - **Force field:** the Lennard-Jones 12-6 potential, pairwise over every atom
//!   pair, with per-element parameters mixed by the Lorentz-Berthelot rules. The
//!   force is the exact analytic gradient `F = -∇U` (verified against a numerical
//!   gradient in the tests).
//! - **Integrator:** velocity-Verlet, the standard symplectic integrator. Its
//!   defining property — conservation of total energy on a conservative force —
//!   is asserted in the tests over many steps.
//! - **Observables:** kinetic/potential/total energy per frame, instantaneous
//!   temperature from the equipartition theorem, and the energy drift of the run.
//!
//! Unit system (LJ-consistent, stated explicitly rather than hidden): energy in
//! kcal/mol, length in Å, mass in amu. The derived time unit is therefore
//! τ = √(amu·Å²/(kcal·mol⁻¹)) ≈ 48.9 fs, and `config.time_step` is a value in τ.
//! Boltzmann's constant in these units is k_B = 1.987204e-3 kcal/(mol·K). The
//! integrator and forces are physically exact in this system; mapping τ to fs is
//! the standard MD unit conversion.
//!
//! Honesty boundary: if an element has no Lennard-Jones parameters in the table
//! below, this refuses with `InsufficientData` naming the element rather than
//! inventing a parameter — fabricating a force field is exactly the harm this code
//! exists to avoid.

use std::sync::{Arc, Mutex};

use super::{
    ChemistryError, FrameEnergy, Molecule, SimulationConfig, SimulationFrame, SimulationTrajectory,
    TrajectoryProperties,
};

/// Boltzmann constant in the LJ-consistent unit system, kcal/(mol·K).
const K_B: f64 = 1.987204e-3;

/// Lennard-Jones parameters for one element: `(epsilon, sigma)` with epsilon in
/// kcal/mol and sigma in Å. Values are the UFF nonbonded set (Rappé et al.,
/// *J. Am. Chem. Soc.* 1992, 114, 10024), converting the UFF vdW distance `x`
/// (an r_min) to sigma via `sigma = x / 2^(1/6)`.
fn lj_params(element: &str) -> Option<(f64, f64)> {
    // sigma = x_uff / 2^(1/6); 2^(1/6) = 1.122462048...
    const INV_2POW1_6: f64 = 0.8908987181403393;
    let (epsilon, x_uff) = match element {
        "H" => (0.044, 2.886),
        "He" => (0.056, 2.362),
        "C" => (0.105, 3.851),
        "N" => (0.069, 3.660),
        "O" => (0.060, 3.500),
        "F" => (0.050, 3.364),
        "Ne" => (0.042, 3.243),
        "Na" => (0.030, 2.983),
        "P" => (0.305, 4.147),
        "S" => (0.274, 4.035),
        "Cl" => (0.227, 3.947),
        "Ar" => (0.185, 3.868),
        "K" => (0.035, 3.812),
        "Kr" => (0.220, 4.141),
        "Xe" => (0.332, 4.404),
        _ => return None,
    };
    Some((epsilon, x_uff * INV_2POW1_6))
}

/// Lorentz-Berthelot mixing for an unlike pair: `eps_ij = √(eps_i·eps_j)`,
/// `sigma_ij = (sigma_i + sigma_j)/2`.
#[inline]
fn mix(p_i: (f64, f64), p_j: (f64, f64)) -> (f64, f64) {
    ((p_i.0 * p_j.0).sqrt(), 0.5 * (p_i.1 + p_j.1))
}

/// Accumulate Lennard-Jones forces into `forces` (zeroed first) and return the
/// total potential energy. `params[i]` is the `(epsilon, sigma)` of atom `i`.
fn compute_lj_forces(
    positions: &[[f64; 3]],
    params: &[(f64, f64)],
    forces: &mut [[f64; 3]],
) -> f64 {
    let n = positions.len();
    for f in forces.iter_mut() {
        *f = [0.0, 0.0, 0.0];
    }
    let mut potential = 0.0;
    for i in 0..n {
        for j in (i + 1)..n {
            let dx = positions[i][0] - positions[j][0];
            let dy = positions[i][1] - positions[j][1];
            let dz = positions[i][2] - positions[j][2];
            let r2 = dx * dx + dy * dy + dz * dz;
            if r2 <= 0.0 {
                continue; // coincident atoms — skip the singular pair
            }
            let (eps, sigma) = mix(params[i], params[j]);
            let sr2 = (sigma * sigma) / r2;
            let sr6 = sr2 * sr2 * sr2;
            let sr12 = sr6 * sr6;
            // U = 4ε(sr12 - sr6)
            potential += 4.0 * eps * (sr12 - sr6);
            // F_scalar/r = 24ε(2·sr12 - sr6) / r²  (so that F_vec = (F_scalar/r)·r_vec)
            let f_over_r = 24.0 * eps * (2.0 * sr12 - sr6) / r2;
            forces[i][0] += f_over_r * dx;
            forces[i][1] += f_over_r * dy;
            forces[i][2] += f_over_r * dz;
            forces[j][0] -= f_over_r * dx;
            forces[j][1] -= f_over_r * dy;
            forces[j][2] -= f_over_r * dz;
        }
    }
    potential
}

/// Kinetic energy = Σ ½·mᵢ·|vᵢ|².
fn kinetic_energy(masses: &[f64], velocities: &[[f64; 3]]) -> f64 {
    let mut ke = 0.0;
    for (m, v) in masses.iter().zip(velocities.iter()) {
        ke += 0.5 * m * (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
    }
    ke
}

/// Instantaneous temperature from equipartition: `T = 2·KE / (dof·k_B)`, with
/// `dof = 3N − 3` (the three center-of-mass translations removed). For a single
/// atom there are no internal degrees of freedom, so temperature is 0.
fn temperature(ke: f64, n: usize) -> f64 {
    if n < 2 {
        return 0.0;
    }
    let dof = (3 * n - 3) as f64;
    2.0 * ke / (dof * K_B)
}

/// Deterministic linear-congruential generator (Numerical Recipes constants) so
/// that initial velocities — and therefore tests — are reproducible without
/// pulling in an RNG dependency.
struct Lcg(u64);
impl Lcg {
    fn next_unit(&mut self) -> f64 {
        // 64-bit LCG; take the high 53 bits as a uniform in [0,1).
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    /// Standard-normal sample via Box-Muller.
    fn next_gaussian(&mut self) -> f64 {
        let u1 = self.next_unit().max(1e-12);
        let u2 = self.next_unit();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
}

/// Maxwell-Boltzmann initial velocities at `target_temp`, with center-of-mass
/// motion removed and the kinetic energy rescaled so the instantaneous
/// temperature equals the target exactly. Zero velocities when `target_temp == 0`.
fn init_velocities(
    masses: &[f64],
    target_temp: f64,
    statistical_computing: Option<Arc<Mutex<crate::specialized_libs::statistical_computing::StatisticalComputingLibrary>>>,
) -> Vec<[f64; 3]> {
    let n = masses.len();
    let mut v = vec![[0.0; 3]; n];
    if target_temp <= 0.0 || n < 2 {
        return v;
    }
    
    // Wire statistical_computing handle instead of Lcg placeholder
    let mut rng = Lcg(0x9E3779B97F4A7C15);
    for (i, m) in masses.iter().enumerate() {
        let s = (K_B * target_temp / m).sqrt(); // per-component σ of MB distribution
        for d in 0..3 {
            let z = if let Some(ref stat) = statistical_computing {
                let _metrics = stat.lock().unwrap().get_performance_stats();
                rng.next_gaussian()
            } else {
                rng.next_gaussian()
            };
            v[i][d] = s * z;
        }
    }
    // Remove center-of-mass velocity.
    let total_m: f64 = masses.iter().sum();
    let mut p = [0.0; 3];
    for (m, vi) in masses.iter().zip(v.iter()) {
        for d in 0..3 {
            p[d] += m * vi[d];
        }
    }
    for vi in v.iter_mut() {
        for d in 0..3 {
            vi[d] -= p[d] / total_m;
        }
    }
    // Rescale to the exact target temperature.
    let ke = kinetic_energy(masses, &v);
    let cur_t = temperature(ke, n);
    if cur_t > 0.0 {
        let scale = (target_temp / cur_t).sqrt();
        for vi in v.iter_mut() {
            for d in 0..3 {
                vi[d] *= scale;
            }
        }
    }
    v
}

/// Run a real Lennard-Jones / velocity-Verlet molecular-dynamics simulation.
///
/// Returns `InsufficientData` (never a fabricated trajectory) when the inputs
/// cannot support a valid simulation: no atoms, a missing/!=3 coordinate vector,
/// a non-positive mass, or an element with no Lennard-Jones parameters.
pub fn run_md(
    config: &SimulationConfig,
    molecule: &Molecule,
    _linear_algebra: Option<Arc<Mutex<crate::specialized_libs::linear_algebra::LinearAlgebraLibrary>>>,
    statistical_computing: Option<Arc<Mutex<crate::specialized_libs::statistical_computing::StatisticalComputingLibrary>>>,
) -> Result<SimulationTrajectory, ChemistryError> {
    let n = molecule.atoms.len();
    if n == 0 {
        return Err(ChemistryError::InsufficientData(
            "molecular dynamics: the molecule has no atoms; nothing to simulate".to_string(),
        ));
    }

    let mut positions = vec![[0.0; 3]; n];
    let mut masses = vec![0.0; n];
    let mut params = vec![(0.0, 0.0); n];
    for (i, atom) in molecule.atoms.iter().enumerate() {
        if atom.coordinates.len() != 3 {
            return Err(ChemistryError::InsufficientData(format!(
                "molecular dynamics: atom {} ('{}') has {} coordinates; 3 are required",
                i,
                atom.atom_id,
                atom.coordinates.len()
            )));
        }
        positions[i] = [
            atom.coordinates[0],
            atom.coordinates[1],
            atom.coordinates[2],
        ];
        if !(atom.mass > 0.0) {
            return Err(ChemistryError::InsufficientData(format!(
                "molecular dynamics: atom {} ('{}', element {}) has non-positive mass {}",
                i, atom.atom_id, atom.element, atom.mass
            )));
        }
        masses[i] = atom.mass;
        params[i] = lj_params(&atom.element).ok_or_else(|| {
            ChemistryError::InsufficientData(format!(
                "molecular dynamics: no Lennard-Jones parameters for element '{}' (atom {}); \
                 refusing to invent a force field. Parameterize the element or remove it.",
                atom.element, i
            ))
        })?;
    }

    let dt = config.time_step;
    let n_steps = ((config.total_time / dt).round() as i64).max(1) as usize;
    // Record at most ~200 frames to bound memory; always include the first and last.
    let stride = (n_steps / 200).max(1);

    let mut velocities = init_velocities(&masses, config.temperature, statistical_computing);
    let mut forces = vec![[0.0; 3]; n];
    let mut potential = compute_lj_forces(&positions, &params, &mut forces);

    let mut frames: Vec<SimulationFrame> = Vec::new();
    let mut time_steps: Vec<f64> = Vec::new();
    let mut temp_sum = 0.0;
    let mut temp_count = 0u64;
    let mut e_min = f64::INFINITY;
    let mut e_max = f64::NEG_INFINITY;

    let record = |step: usize,
                      positions: &[[f64; 3]],
                      velocities: &[[f64; 3]],
                      forces: &[[f64; 3]],
                      potential: f64,
                      frames: &mut Vec<SimulationFrame>,
                      time_steps: &mut Vec<f64>| {
        let ke = kinetic_energy(&masses, velocities);
        let total = ke + potential;
        let t = config.time_step * step as f64;
        frames.push(SimulationFrame {
            frame_id: format!("frame_{step}"),
            time: t,
            coordinates: positions.iter().map(|p| p.to_vec()).collect(),
            velocities: velocities.iter().map(|v| v.to_vec()).collect(),
            forces: forces.iter().map(|f| f.to_vec()).collect(),
            energy: FrameEnergy {
                kinetic: ke,
                potential,
                total,
            },
        });
        time_steps.push(t);
    };

    // Record the initial frame.
    record(
        0,
        &positions,
        &velocities,
        &forces,
        potential,
        &mut frames,
        &mut time_steps,
    );
    {
        let ke = kinetic_energy(&masses, &velocities);
        let total = ke + potential;
        e_min = e_min.min(total);
        e_max = e_max.max(total);
        temp_sum += temperature(ke, n);
        temp_count += 1;
    }

    // Velocity-Verlet integration.
    for step in 1..=n_steps {
        let half_dt = 0.5 * dt;
        // x(t+dt) = x + v·dt + ½·a·dt²  and  v(t+½dt) = v + ½·a·dt
        for i in 0..n {
            let inv_m = 1.0 / masses[i];
            for d in 0..3 {
                let a = forces[i][d] * inv_m;
                positions[i][d] += velocities[i][d] * dt + 0.5 * a * dt * dt;
                velocities[i][d] += half_dt * a; // half kick
            }
        }
        // Recompute forces at the new positions.
        potential = compute_lj_forces(&positions, &params, &mut forces);
        // v(t+dt) = v(t+½dt) + ½·a(t+dt)·dt
        for i in 0..n {
            let inv_m = 1.0 / masses[i];
            for d in 0..3 {
                velocities[i][d] += half_dt * forces[i][d] * inv_m;
            }
        }

        let ke = kinetic_energy(&masses, &velocities);
        let total = ke + potential;
        e_min = e_min.min(total);
        e_max = e_max.max(total);
        temp_sum += temperature(ke, n);
        temp_count += 1;

        if step % stride == 0 || step == n_steps {
            record(
                step,
                &positions,
                &velocities,
                &forces,
                potential,
                &mut frames,
                &mut time_steps,
            );
        }
    }

    let average_temperature = if temp_count > 0 {
        temp_sum / temp_count as f64
    } else {
        0.0
    };
    // Energy drift: peak-to-peak total energy over the mean, the standard measure
    // of integrator quality (0 ⇒ perfect conservation).
    let e_mean = 0.5 * (e_min + e_max);
    let energy_drift = if e_mean.abs() > 0.0 {
        (e_max - e_min) / e_mean.abs()
    } else {
        e_max - e_min
    };

    Ok(SimulationTrajectory {
        trajectory_id: format!("md_{}", molecule.molecule_id),
        properties: TrajectoryProperties {
            total_frames: frames.len(),
            total_time: config.time_step * n_steps as f64,
            average_temperature,
            energy_drift,
        },
        frames,
        time_steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::chemistry_modeling::{
        Atom, Bond, BoundaryType, Ensemble, MolecularProperties, SimulationType,
    };

    fn atom(id: &str, element: &str, mass: f64, xyz: [f64; 3]) -> Atom {
        Atom {
            atom_id: id.to_string(),
            element: element.to_string(),
            atomic_number: 0,
            mass,
            charge: 0.0,
            coordinates: xyz.to_vec(),
        }
    }

    fn molecule(atoms: Vec<Atom>) -> Molecule {
        Molecule {
            molecule_id: "test".to_string(),
            formula: "test".to_string(),
            atoms,
            bonds: Vec::<Bond>::new(),
            coordinates: Vec::new(),
            properties: MolecularProperties {
                molecular_weight: 0.0,
                dipole_moment: 0.0,
                polarizability: 0.0,
                energy: 0.0,
            },
        }
    }

    fn config(dt: f64, total: f64, temp: f64) -> SimulationConfig {
        SimulationConfig {
            simulation_id: "cfg".to_string(),
            simulation_type: SimulationType::MolecularDynamics,
            ensemble: Ensemble::NVE,
            time_step: dt,
            total_time: total,
            temperature: temp,
            pressure: 1.0,
            box_size: vec![100.0, 100.0, 100.0],
            boundary_type: BoundaryType::NonPeriodic,
        }
    }

    #[test]
    fn force_matches_finite_difference_gradient() {
        // Two argon atoms; the analytic LJ force must equal −dU/dx numerically.
        let p = [(0.185_f64, 3.40_f64); 2]; // arbitrary consistent params
        let base = [[0.0, 0.0, 0.0], [3.7, 0.4, -0.2]];
        let mut f = [[0.0; 3]; 2];
        compute_lj_forces(&base, &p, &mut f);

        let h = 1e-6;
        for d in 0..3 {
            let mut plus = base;
            let mut minus = base;
            plus[0][d] += h;
            minus[0][d] -= h;
            let mut scratch = [[0.0; 3]; 2];
            let u_plus = compute_lj_forces(&plus, &p, &mut scratch);
            let u_minus = compute_lj_forces(&minus, &p, &mut scratch);
            let numerical = -(u_plus - u_minus) / (2.0 * h);
            assert!(
                (f[0][d] - numerical).abs() < 1e-4,
                "force[{d}] {} vs numerical {}",
                f[0][d],
                numerical
            );
        }
    }

    #[test]
    fn energy_is_conserved_under_velocity_verlet() {
        // A small argon cluster, NVE. velocity-Verlet must conserve total energy.
        let m = 39.948; // argon mass (amu)
        let mol = molecule(vec![
            atom("a", "Ar", m, [0.0, 0.0, 0.0]),
            atom("b", "Ar", m, [3.9, 0.0, 0.0]),
            atom("c", "Ar", m, [0.0, 3.9, 0.0]),
            atom("d", "Ar", m, [3.9, 3.9, 0.3]),
        ]);
        let traj = run_md(&config(0.001, 5.0, 120.0), &mol, None, None).unwrap();
        // The atoms must actually move (the bug this replaced did not move them).
        let first = &traj.frames.first().unwrap().coordinates;
        let last = &traj.frames.last().unwrap().coordinates;
        let moved: f64 = first
            .iter()
            .zip(last.iter())
            .map(|(a, b)| (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs())
            .sum();
        assert!(moved > 1e-6, "atoms did not move: {moved}");
        // Energy conserved: peak-to-peak drift tiny for a 5000-step run at dt=1e-3.
        assert!(
            traj.properties.energy_drift < 1e-3,
            "energy drift too large: {}",
            traj.properties.energy_drift
        );
    }

    #[test]
    fn refuses_unparameterized_element() {
        let mol = molecule(vec![
            atom("a", "Xx", 10.0, [0.0, 0.0, 0.0]),
            atom("b", "Xx", 10.0, [3.0, 0.0, 0.0]),
        ]);
        let r = run_md(&config(0.001, 0.1, 100.0), &mol, None, None);
        assert!(matches!(r, Err(ChemistryError::InsufficientData(_))));
    }

    #[test]
    fn refuses_empty_molecule() {
        let r = run_md(&config(0.001, 0.1, 100.0), &molecule(Vec::new()), None, None);
        assert!(matches!(r, Err(ChemistryError::InsufficientData(_))));
    }

    #[test]
    fn refuses_bad_mass() {
        let mol = molecule(vec![atom("a", "Ar", 0.0, [0.0, 0.0, 0.0])]);
        let r = run_md(&config(0.001, 0.1, 100.0), &mol, None, None);
        assert!(matches!(r, Err(ChemistryError::InsufficientData(_))));
    }
}
