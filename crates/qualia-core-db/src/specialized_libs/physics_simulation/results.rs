use super::*;

/// Physics simulation result
#[derive(Debug, Clone)]
pub struct PhysicsSimulationResult<T> {
    pub result: T,
    pub simulation_time: u64,
    pub solver_time: u64,
    pub data_time: u64,
    pub convergence_info: ConvergenceInfo,
    pub performance_info: PerformanceInfo,
}

/// Convergence information
#[derive(Debug, Clone)]
pub struct ConvergenceInfo {
    pub converged: bool,
    pub iterations: u32,
    pub residual_norm: f64,
    pub convergence_rate: f64,
    pub final_error: f64,
}

/// Performance information
#[derive(Debug, Clone)]
pub struct PerformanceInfo {
    pub cpu_utilization: f64,
    pub memory_utilization: f64,
    pub network_utilization: f64,
    pub io_utilization: f64,
    pub parallel_efficiency: f64,
}


// ============================================================================
// Genuine simulations for the declared `SimulationType` domains.
//
// Every method below marshals an initial state into slices and hands the actual
// time-integration / eigenproblem to a tested solver in `crate::solvers`:
//   * `integrate_dopri5`     — adaptive Dormand–Prince RK45 (vector ODE systems)
//   * `integrate_symplectic` — Störmer–Verlet / Ruth / Yoshida (separable Hamiltonians)
//   * `symmetric_eigen`      — cyclic-Jacobi symmetric eigensolver
// The physics (forces, Laplacians, Hamiltonians) is set up here; the numerics are not.
// ============================================================================

/// Trajectory + landing diagnostics for `run_projectile_motion` (ParticlePhysics).
#[derive(Debug, Clone)]
pub struct ProjectileResult {
    /// Sampled `[t, x, y, vx, vy]` rows along the flight.
    pub trajectory: Vec<[f64; 5]>,
    /// Horizontal distance at ground return (interpolated y=0). No-drag: v0²·sin(2θ)/g.
    pub range: f64,
    /// Peak height reached.
    pub max_height: f64,
    /// Time of flight to ground return (interpolated).
    pub time_of_flight: f64,
    /// Whether the projectile returned to y=0 within `max_time`.
    pub landed: bool,
    pub steps_accepted: u32,
    pub steps_rejected: u32,
}

/// Result of `run_harmonic_oscillator` (StructuralDynamics / spring–mass), symplectic.
#[derive(Debug, Clone)]
pub struct OscillatorResult {
    pub times: Vec<f64>,
    pub positions: Vec<f64>,
    pub velocities: Vec<f64>,
    /// Analytic period 2π·√(m/k).
    pub analytic_period: f64,
    /// Period measured from the integrated trajectory (mean crossing interval).
    pub measured_period: f64,
    pub energy_initial: f64,
    pub energy_final: f64,
    /// Max |E−E₀| over the run — the bounded symplectic energy drift.
    pub max_energy_drift: f64,
}

/// Result of `run_pendulum` (nonlinear rigid-body dynamics).
#[derive(Debug, Clone)]
pub struct PendulumResult {
    pub times: Vec<f64>,
    pub angles: Vec<f64>,
    pub angular_velocities: Vec<f64>,
    /// Small-angle period 2π·√(L/g).
    pub small_angle_period: f64,
    pub measured_period: f64,
    pub energy_initial: f64,
    pub energy_final: f64,
    pub steps_accepted: u32,
    pub steps_rejected: u32,
}

/// Result of `run_nbody_gravitation` (Astrophysics), direct-sum Newtonian gravity.
#[derive(Debug, Clone)]
pub struct NBodyResult {
    pub num_bodies: usize,
    pub times: Vec<f64>,
    /// Position snapshots; each is the flat `[x0,y0,x1,y1,…]` vector at that time.
    pub position_snapshots: Vec<Vec<f64>>,
    pub final_positions: Vec<f64>,
    pub final_velocities: Vec<f64>,
    pub energy_initial: f64,
    pub energy_final: f64,
    /// |E_final − E_initial| / |E_initial|.
    pub energy_drift_rel: f64,
    pub angular_momentum_initial: f64,
    pub angular_momentum_final: f64,
    pub steps_accepted: u32,
    pub steps_rejected: u32,
}

/// Result of `run_heat_diffusion_1d` (HeatTransfer), insulated (Neumann) ends.
#[derive(Debug, Clone)]
pub struct HeatDiffusionResult {
    pub times: Vec<f64>,
    pub snapshots: Vec<Vec<f64>>,
    pub final_temperature: Vec<f64>,
    pub initial_mean: f64,
    pub final_mean: f64,
    /// max_i |u_i − mean| in the final field (→ 0 as the profile relaxes).
    pub max_deviation_from_mean: f64,
    pub steps_accepted: u32,
    pub steps_rejected: u32,
}

/// Result of `run_wave_equation_1d` (CEM — 1D scalar wave / plane-wave field), fixed ends.
#[derive(Debug, Clone)]
pub struct WaveResult {
    pub times: Vec<f64>,
    /// Displacement (field) snapshots.
    pub snapshots: Vec<Vec<f64>>,
    pub final_displacement: Vec<f64>,
    pub energy_initial: f64,
    pub energy_final: f64,
    pub steps_accepted: u32,
    pub steps_rejected: u32,
}

/// Result of `run_molecular_dynamics` (MolecularDynamics), Lennard-Jones in 2D.
#[derive(Debug, Clone)]
pub struct MolecularDynamicsResult {
    pub num_particles: usize,
    pub times: Vec<f64>,
    pub final_positions: Vec<f64>,
    pub final_velocities: Vec<f64>,
    pub energy_initial: f64,
    pub energy_final: f64,
    pub energy_drift_rel: f64,
    /// Instantaneous kinetic temperature (reduced units, kB=1): 2·KE/dof.
    pub temperature: f64,
    pub steps_accepted: u32,
    pub steps_rejected: u32,
}

/// Result of `run_quantum_stationary_states_1d` (QuantumMechanics), finite-difference
/// time-independent Schrödinger eigenproblem solved by `symmetric_eigen`.
#[derive(Debug, Clone)]
pub struct QuantumSpectrumResult {
    /// Lowest `num_levels` energy eigenvalues, ascending.
    pub eigenvalues: Vec<f64>,
    pub num_grid_points: usize,
    pub dx: f64,
}

/// Result of `run_logistic_growth` (Biophysics — population dynamics).
#[derive(Debug, Clone)]
pub struct PopulationDynamicsResult {
    pub times: Vec<f64>,
    pub population: Vec<f64>,
    pub carrying_capacity: f64,
    pub growth_rate: f64,
    pub steps_accepted: u32,
    pub steps_rejected: u32,
}

/// Result of `run_advection_diffusion_1d` (MultiPhysics — coupled transport + diffusion).
#[derive(Debug, Clone)]
pub struct AdvectionDiffusionResult {
    pub times: Vec<f64>,
    pub snapshots: Vec<Vec<f64>>,
    pub final_field: Vec<f64>,
    pub advection_velocity: f64,
    pub diffusion_coeff: f64,
    /// Σ_i u_i·dx at t=0 and t=end (conserved under the periodic scheme).
    pub initial_total: f64,
    pub final_total: f64,
    pub steps_accepted: u32,
    pub steps_rejected: u32,
}

/// Simulation result
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub node_id: String,
    pub fields: Vec<PhysicsField>,
    pub convergence_info: ConvergenceInfo,
    pub performance_info: PerformanceInfo,
}
