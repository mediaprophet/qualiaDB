use super::*;

pub fn ode_solve(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::physics_simulation::{
        CommunicationPattern, DomainDecomposition, DomainType, LoadBalancing, NumericalMethod,
        ParallelConfig, PhysicsSimulationLibrary, SimulationConfig, SimulationType,
        SpatialResolution,
    };

    let v = parse_tool_args(args)?;
    let sim_type = json_str(&v, "type", "cfd");
    let mut lib = PhysicsSimulationLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;

    // Newly-wired analytic/ODE-backed simulations. Each marshals JSON params
    // into the library method (which delegates to the canonical solvers) and
    // returns the salient results. Array inputs are read as JSON number arrays.
    let f64_array = |key: &str| -> Vec<f64> {
        v.get(key)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_f64).collect())
            .unwrap_or_default()
    };
    let samples = v.get("num_samples").and_then(Value::as_u64).unwrap_or(64) as usize;
    let sim_total_time = json_f64(&v, "total_time", 1.0);
    match sim_type {
        "projectile" | "particle" => {
            let r = lib
                .run_projectile_motion(
                    json_f64(&v, "v0", 10.0),
                    json_f64(&v, "angle_rad", std::f64::consts::FRAC_PI_4),
                    json_f64(&v, "g", 9.81),
                    json_f64(&v, "drag", 0.0),
                    samples,
                    json_f64(&v, "max_time", 10.0),
                )
                .map_err(|_| McpSystemError::InvalidParameters)?;
            return Ok(json!({
                "type": "projectile", "range": r.range, "max_height": r.max_height,
                "time_of_flight": r.time_of_flight, "landed": r.landed,
                "steps_accepted": r.steps_accepted
            })
            .to_string());
        }
        "harmonic_oscillator" | "structural_dynamics" => {
            let r = lib
                .run_harmonic_oscillator(
                    json_f64(&v, "mass", 1.0),
                    json_f64(&v, "k_spring", 1.0),
                    json_f64(&v, "x0", 1.0),
                    json_f64(&v, "v0", 0.0),
                    sim_total_time,
                    samples,
                )
                .map_err(|_| McpSystemError::InvalidParameters)?;
            return Ok(json!({
                "type": "harmonic_oscillator", "analytic_period": r.analytic_period,
                "measured_period": r.measured_period, "energy_initial": r.energy_initial,
                "energy_final": r.energy_final, "max_energy_drift": r.max_energy_drift
            })
            .to_string());
        }
        "pendulum" => {
            let r = lib
                .run_pendulum(
                    json_f64(&v, "length", 1.0),
                    json_f64(&v, "g", 9.81),
                    json_f64(&v, "theta0", 0.2),
                    json_f64(&v, "omega0", 0.0),
                    sim_total_time,
                    samples,
                )
                .map_err(|_| McpSystemError::InvalidParameters)?;
            return Ok(json!({
                "type": "pendulum", "small_angle_period": r.small_angle_period,
                "measured_period": r.measured_period, "energy_initial": r.energy_initial,
                "energy_final": r.energy_final
            })
            .to_string());
        }
        "nbody" | "astrophysics" | "gravitation" => {
            let r = lib
                .run_nbody_gravitation(
                    f64_array("masses"),
                    f64_array("positions"),
                    f64_array("velocities"),
                    json_f64(&v, "g", 1.0),
                    json_f64(&v, "softening", 1e-3),
                    sim_total_time,
                    samples,
                )
                .map_err(|_| McpSystemError::InvalidParameters)?;
            return Ok(json!({
                "type": "nbody", "num_bodies": r.num_bodies,
                "energy_initial": r.energy_initial, "energy_final": r.energy_final,
                "energy_drift_rel": r.energy_drift_rel,
                "final_positions": r.final_positions
            })
            .to_string());
        }
        "heat_diffusion" | "heat_transfer" => {
            let r = lib
                .run_heat_diffusion_1d(
                    f64_array("initial"),
                    json_f64(&v, "alpha", 1.0),
                    json_f64(&v, "dx", 0.1),
                    sim_total_time,
                    samples,
                )
                .map_err(|_| McpSystemError::InvalidParameters)?;
            return Ok(json!({
                "type": "heat_diffusion", "initial_mean": r.initial_mean,
                "final_mean": r.final_mean, "max_deviation_from_mean": r.max_deviation_from_mean,
                "final_temperature": r.final_temperature
            })
            .to_string());
        }
        "wave" | "cem" => {
            let r = lib
                .run_wave_equation_1d(
                    f64_array("initial_displacement"),
                    f64_array("initial_velocity"),
                    json_f64(&v, "c", 1.0),
                    json_f64(&v, "dx", 0.1),
                    sim_total_time,
                    samples,
                )
                .map_err(|_| McpSystemError::InvalidParameters)?;
            return Ok(json!({
                "type": "wave", "energy_initial": r.energy_initial,
                "energy_final": r.energy_final, "final_displacement": r.final_displacement
            })
            .to_string());
        }
        "quantum" | "quantum_mechanics" => {
            let r = lib
                .run_quantum_stationary_states_1d(
                    f64_array("potential"),
                    json_f64(&v, "dx", 0.1),
                    json_f64(&v, "mass", 1.0),
                    json_f64(&v, "hbar", 1.0),
                    v.get("num_levels").and_then(Value::as_u64).unwrap_or(3) as usize,
                )
                .map_err(|_| McpSystemError::InvalidParameters)?;
            return Ok(json!({
                "type": "quantum", "eigenvalues": r.eigenvalues,
                "num_grid_points": r.num_grid_points
            })
            .to_string());
        }
        "logistic_growth" | "biophysics" => {
            let r = lib
                .run_logistic_growth(
                    json_f64(&v, "n0", 1.0),
                    json_f64(&v, "growth_rate", 0.5),
                    json_f64(&v, "carrying_capacity", 100.0),
                    sim_total_time,
                    samples,
                )
                .map_err(|_| McpSystemError::InvalidParameters)?;
            return Ok(json!({
                "type": "logistic_growth", "population": r.population,
                "carrying_capacity": r.carrying_capacity, "growth_rate": r.growth_rate
            })
            .to_string());
        }
        "advection_diffusion" | "multiphysics" => {
            let r = lib
                .run_advection_diffusion_1d(
                    f64_array("initial"),
                    json_f64(&v, "advection_velocity", 1.0),
                    json_f64(&v, "diffusion_coeff", 0.1),
                    json_f64(&v, "dx", 0.1),
                    sim_total_time,
                    samples,
                )
                .map_err(|_| McpSystemError::InvalidParameters)?;
            return Ok(json!({
                "type": "advection_diffusion", "initial_total": r.initial_total,
                "final_total": r.final_total, "final_field": r.final_field
            })
            .to_string());
        }
        "molecular_dynamics" if v.get("positions").is_some() => {
            let r = lib
                .run_molecular_dynamics(
                    f64_array("positions"),
                    f64_array("velocities"),
                    json_f64(&v, "epsilon", 1.0),
                    json_f64(&v, "sigma", 1.0),
                    json_f64(&v, "mass", 1.0),
                    sim_total_time,
                    samples,
                )
                .map_err(|_| McpSystemError::InvalidParameters)?;
            return Ok(json!({
                "type": "molecular_dynamics", "num_particles": r.num_particles,
                "energy_initial": r.energy_initial, "energy_final": r.energy_final,
                "energy_drift_rel": r.energy_drift_rel, "temperature": r.temperature
            })
            .to_string());
        }
        _ => {}
    }

    let nx = v.get("nx").and_then(Value::as_u64).unwrap_or(10) as usize;
    let ny = v.get("ny").and_then(Value::as_u64).unwrap_or(10) as usize;
    let dx = json_f64(&v, "dx", 0.1);
    let time_step = json_f64(&v, "time_step", 0.001);
    let total_time = json_f64(&v, "total_time", 0.01);
    let simulation_id = v
        .get("simulation_id")
        .and_then(Value::as_str)
        .unwrap_or("mcp_sim")
        .to_string();

    let config = SimulationConfig {
        simulation_id,
        simulation_type: if sim_type == "distributed" || sim_type == "molecular_dynamics" {
            SimulationType::MolecularDynamics
        } else {
            SimulationType::CFD
        },
        domain_type: DomainType::TwoDimensional,
        time_step,
        total_time,
        spatial_resolution: SpatialResolution {
            nx,
            ny: Some(ny),
            nz: None,
            dx,
            dy: Some(json_f64(&v, "dy", dx)),
            dz: None,
        },
        numerical_method: NumericalMethod::FiniteVolume,
        parallel_config: ParallelConfig {
            num_threads: v.get("num_threads").and_then(Value::as_u64).unwrap_or(1) as usize,
            num_processes: 1,
            domain_decomposition: DomainDecomposition::TwoDimensional,
            load_balancing: LoadBalancing::Dynamic,
            communication_pattern: CommunicationPattern::Hybrid,
        },
    };

    let mut sim = lib
        .create_simulation(config)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let r = lib
        .run_cfd_simulation(&mut sim)
        .map_err(|_| McpSystemError::InvalidParameters)?;

    Ok(json!({
        "field_count": r.result.len(),
        "converged": r.convergence_info.converged,
        "iterations": r.convergence_info.iterations,
        "final_error": r.convergence_info.final_error
    })
    .to_string())
}
