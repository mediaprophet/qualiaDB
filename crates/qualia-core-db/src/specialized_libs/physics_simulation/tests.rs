use super::*;
#[test]
fn test_physics_library_creation() {
    let mut library = PhysicsSimulationLibrary::new();
    assert!(library.initialize().is_ok());
}

#[test]
fn test_simulation_creation() {
    let mut library = PhysicsSimulationLibrary::new();
    library.initialize().unwrap();

    let config = SimulationConfig {
        simulation_id: "test_simulation".to_string(),
        simulation_type: SimulationType::CFD,
        domain_type: DomainType::ThreeDimensional,
        time_step: 0.001,
        total_time: 1.0,
        spatial_resolution: SpatialResolution {
            nx: 100,
            ny: Some(100),
            nz: Some(100),
            dx: 0.01,
            dy: Some(0.01),
            dz: Some(0.01),
        },
        numerical_method: NumericalMethod::FiniteVolume,
        parallel_config: ParallelConfig {
            num_threads: 4,
            num_processes: 2,
            domain_decomposition: DomainDecomposition::ThreeDimensional,
            load_balancing: LoadBalancing::Dynamic,
            communication_pattern: CommunicationPattern::Hybrid,
        },
    };

    let simulation = library.create_simulation(config).unwrap();

    assert_eq!(simulation.config.simulation_id, "test_simulation");
    assert_eq!(simulation.config.simulation_type, SimulationType::CFD);
    assert_eq!(simulation.config.domain_type, DomainType::ThreeDimensional);
    assert_eq!(simulation.config.time_step, 0.001);
    assert_eq!(simulation.config.total_time, 1.0);
}

#[test]
fn test_cfd_simulation() {
    let mut library = PhysicsSimulationLibrary::new();
    library.initialize().unwrap();

    let config = SimulationConfig {
        simulation_id: "cfd_test".to_string(),
        simulation_type: SimulationType::CFD,
        domain_type: DomainType::TwoDimensional,
        time_step: 0.001,
        total_time: 0.1,
        spatial_resolution: SpatialResolution {
            nx: 50,
            ny: Some(50),
            nz: None,
            dx: 0.02,
            dy: Some(0.02),
            dz: None,
        },
        numerical_method: NumericalMethod::FiniteVolume,
        parallel_config: ParallelConfig {
            num_threads: 2,
            num_processes: 1,
            domain_decomposition: DomainDecomposition::TwoDimensional,
            load_balancing: LoadBalancing::Dynamic,
            communication_pattern: CommunicationPattern::Hybrid,
        },
    };

    let mut simulation = library.create_simulation(config).unwrap();

    let result = library.run_cfd_simulation(&mut simulation).unwrap();

    // Real Burgers integration: assert the actual computed behaviour, not a fabricated
    // "converged". The fields must be present, finite, and non-trivially evolved; the
    // residual is a real, finite, computed number.
    assert_eq!(result.result.len(), 3); // velocity, pressure, temperature
    assert!(result.convergence_info.iterations > 0);
    assert!(result.convergence_info.residual_norm.is_finite());
    assert!(result.convergence_info.residual_norm >= 0.0);
    let velocity = &result.result[0].data;
    assert!(velocity.iter().all(|v| v.is_finite()));
    assert!(velocity.iter().any(|&v| v.abs() > 0.0)); // the IC actually propagated
                                                      // Pressure tracks Bernoulli: where velocity is non-zero, pressure drops below p_ref.
    let pressure = &result.result[1].data;
    assert!(pressure
        .iter()
        .all(|&p| p.is_finite() && p <= 101_325.0 + 1e-6));
}

// ── Genuine physics simulations wired to solvers/ — known-value tests ──────────

#[test]
fn projectile_no_drag_matches_analytic_range() {
    // Range = v0²·sin(2θ)/g. v0=20, θ=45°, g=9.81 → ~40.7747 m; flight ~2.884 s.
    let lib = PhysicsSimulationLibrary::new();
    let v0 = 20.0;
    let theta = std::f64::consts::FRAC_PI_4;
    let g = 9.81;
    let t_flight = 2.0 * v0 * theta.sin() / g;
    let res = lib
        .run_projectile_motion(v0, theta, g, 0.0, 4000, t_flight * 1.05)
        .unwrap();
    assert!(res.landed);
    let analytic_range = v0 * v0 * (2.0 * theta).sin() / g;
    assert!(
        (res.range - analytic_range).abs() < 0.05,
        "range {} vs analytic {}",
        res.range,
        analytic_range
    );
    // Max height = (v0·sinθ)²/(2g) ≈ 10.19 m.
    let analytic_h = (v0 * theta.sin()).powi(2) / (2.0 * g);
    assert!((res.max_height - analytic_h).abs() < 0.05);
    assert!((res.time_of_flight - t_flight).abs() < 0.02);
}

#[test]
fn projectile_drag_reduces_range() {
    let lib = PhysicsSimulationLibrary::new();
    let (v0, theta, g) = (30.0, std::f64::consts::FRAC_PI_4, 9.81);
    let no_drag = lib
        .run_projectile_motion(v0, theta, g, 0.0, 4000, 10.0)
        .unwrap();
    let with_drag = lib
        .run_projectile_motion(v0, theta, g, 0.02, 4000, 10.0)
        .unwrap();
    assert!(with_drag.landed && no_drag.landed);
    assert!(
        with_drag.range < no_drag.range,
        "drag range {} should be < no-drag range {}",
        with_drag.range,
        no_drag.range
    );
}

#[test]
fn harmonic_oscillator_conserves_energy_and_period() {
    // m=1, k=4 → ω=2, T=π≈3.14159. Symplectic drift must stay tiny.
    let lib = PhysicsSimulationLibrary::new();
    let res = lib
        .run_harmonic_oscillator(1.0, 4.0, 1.0, 0.0, 20.0, 4000)
        .unwrap();
    assert!(
        (res.analytic_period - std::f64::consts::PI).abs() < 1e-9,
        "analytic period {}",
        res.analytic_period
    );
    assert!(
        (res.measured_period - res.analytic_period).abs() < 0.05,
        "measured {} vs analytic {}",
        res.measured_period,
        res.analytic_period
    );
    // Energy at t=0 is ½k x0² = 2.0; drift bounded (symplectic property).
    assert!((res.energy_initial - 2.0).abs() < 1e-9);
    assert!(
        res.max_energy_drift < 1e-3,
        "energy drift {} not bounded",
        res.max_energy_drift
    );
    assert!((res.energy_final - res.energy_initial).abs() < 1e-3);
}

#[test]
fn pendulum_small_angle_period_and_energy() {
    // L=1, g=9.81, θ0=0.05 rad (small) → period ≈ 2π√(L/g) ≈ 2.0064 s.
    let lib = PhysicsSimulationLibrary::new();
    let res = lib.run_pendulum(1.0, 9.81, 0.05, 0.0, 12.0, 6000).unwrap();
    assert!(
        (res.measured_period - res.small_angle_period).abs() < 0.02,
        "measured {} vs small-angle {}",
        res.measured_period,
        res.small_angle_period
    );
    // Energy conserved along the trajectory.
    assert!(
        (res.energy_final - res.energy_initial).abs() < 1e-4,
        "pendulum energy drift {}",
        (res.energy_final - res.energy_initial).abs()
    );
}

#[test]
fn two_body_circular_orbit_stays_circular() {
    // Circular orbit: a heavy body at the origin, a light satellite at radius r with
    // the circular speed v = √(G·M/r). Choose G so that G·M_heavy = 1 with r = 1.
    let lib = PhysicsSimulationLibrary::new();
    let r = 1.0f64;
    let m_heavy = 1000.0f64;
    let gg = 1.0 / m_heavy; // → gg·M_heavy = 1
    let vsat = (gg * m_heavy / r).sqrt(); // = 1.0
                                          // Body 0 = heavy (near-fixed), body 1 = satellite on a circular orbit.
    let masses = vec![m_heavy, 1e-6];
    let positions = vec![0.0, 0.0, r, 0.0];
    let velocities = vec![0.0, 0.0, 0.0, vsat];
    let period = 2.0 * std::f64::consts::PI * r / vsat; // ≈ 6.283
    let res = lib
        .run_nbody_gravitation(masses, positions, velocities, gg, 1e-6, period, 400)
        .unwrap();
    // Radius of the satellite (body 1) about the heavy body stays ~r over one orbit.
    for snap in &res.position_snapshots {
        let rx = snap[2] - snap[0];
        let ry = snap[3] - snap[1];
        let radius = (rx * rx + ry * ry).sqrt();
        assert!(
            (radius - r).abs() < 0.02,
            "orbit radius drifted to {}",
            radius
        );
    }
    // Energy and angular momentum conserved.
    assert!(
        res.energy_drift_rel < 1e-3,
        "energy drift {}",
        res.energy_drift_rel
    );
    assert!(
        (res.angular_momentum_final - res.angular_momentum_initial).abs() < 1e-6,
        "L drift {}",
        (res.angular_momentum_final - res.angular_momentum_initial).abs()
    );
}

#[test]
fn heat_diffusion_relaxes_toward_mean_and_conserves_heat() {
    // Insulated bar: total heat conserved, profile flattens toward the mean.
    let lib = PhysicsSimulationLibrary::new();
    let n = 21;
    let dx = 1.0 / (n as f64 - 1.0);
    // Initial: a sine bump (mean ≈ nonzero) with clear spatial variation.
    let initial: Vec<f64> = (0..n)
        .map(|i| 1.0 + (std::f64::consts::PI * i as f64 * dx).sin())
        .collect();
    let init_max_dev = {
        let mean = initial.iter().sum::<f64>() / n as f64;
        initial
            .iter()
            .map(|&v| (v - mean).abs())
            .fold(0.0, f64::max)
    };
    let res = lib
        .run_heat_diffusion_1d(initial, 1.0, dx, 0.5, 50)
        .unwrap();
    // Total heat conserved (Neumann BC).
    assert!(
        (res.final_mean - res.initial_mean).abs() < 1e-6,
        "mean changed: {} -> {}",
        res.initial_mean,
        res.final_mean
    );
    // Profile relaxed substantially toward the mean.
    assert!(
        res.max_deviation_from_mean < 0.4 * init_max_dev,
        "deviation {} did not relax from {}",
        res.max_deviation_from_mean,
        init_max_dev
    );
}

#[test]
fn wave_equation_conserves_energy() {
    let lib = PhysicsSimulationLibrary::new();
    let n = 41;
    let dx = 1.0 / (n as f64 - 1.0);
    // Plucked string: sine mode, at rest.
    let u0: Vec<f64> = (0..n)
        .map(|i| (std::f64::consts::PI * i as f64 * dx).sin())
        .collect();
    let v0 = vec![0.0; n];
    let res = lib.run_wave_equation_1d(u0, v0, 1.0, dx, 0.5, 40).unwrap();
    assert!(res.energy_initial > 0.0);
    let rel = (res.energy_final - res.energy_initial).abs() / res.energy_initial;
    assert!(rel < 0.05, "wave energy drift {}", rel);
    // Ends stay pinned.
    assert!(res.final_displacement[0].abs() < 1e-9);
    assert!(res.final_displacement[n - 1].abs() < 1e-9);
}

#[test]
fn molecular_dynamics_conserves_energy() {
    // Two LJ particles released slightly inside the minimum (r0 = 2^(1/6)·σ) oscillate;
    // total energy must be conserved.
    let lib = PhysicsSimulationLibrary::new();
    let sigma = 1.0;
    let r_min = 2f64.powf(1.0 / 6.0) * sigma;
    let r_start = 0.95 * r_min; // compressed → will oscillate
    let positions = vec![0.0, 0.0, r_start, 0.0];
    let velocities = vec![0.0, 0.0, 0.0, 0.0];
    let res = lib
        .run_molecular_dynamics(positions, velocities, 1.0, sigma, 1.0, 2.0, 200)
        .unwrap();
    assert_eq!(res.num_particles, 2);
    assert!(
        res.energy_drift_rel < 1e-3,
        "MD energy drift {}",
        res.energy_drift_rel
    );
}

#[test]
fn molecular_dynamics_rest_at_potential_minimum() {
    // At r = r_min the pair force is zero → particles at rest stay (nearly) put.
    let lib = PhysicsSimulationLibrary::new();
    let sigma = 1.0;
    let r_min = 2f64.powf(1.0 / 6.0) * sigma;
    let positions = vec![0.0, 0.0, r_min, 0.0];
    let velocities = vec![0.0, 0.0, 0.0, 0.0];
    let res = lib
        .run_molecular_dynamics(positions, velocities, 1.0, sigma, 1.0, 1.0, 50)
        .unwrap();
    let sep = res.final_positions[2] - res.final_positions[0];
    assert!(
        (sep - r_min).abs() < 1e-6,
        "separation drifted to {} from r_min {}",
        sep,
        r_min
    );
}

#[test]
fn quantum_infinite_well_matches_discrete_spectrum() {
    // Infinite square well, n interior points, walls implicit at the ends. The FD
    // Hamiltonian's exact eigenvalues are E_k = 2t·(1 − cos(kπ/(n+1))), t=ħ²/(2m dx²).
    let lib = PhysicsSimulationLibrary::new();
    let n = 100usize;
    let width = 1.0f64;
    let dx = width / (n as f64 + 1.0); // walls at 0 and width
    let (mass, hbar) = (1.0, 1.0);
    let potential = vec![0.0f64; n];
    let res = lib
        .run_quantum_stationary_states_1d(potential, dx, mass, hbar, 3)
        .unwrap();
    let t = hbar * hbar / (2.0 * mass * dx * dx);
    for k in 1..=3usize {
        let exact = 2.0 * t * (1.0 - (k as f64 * std::f64::consts::PI / (n as f64 + 1.0)).cos());
        let got = res.eigenvalues[k - 1];
        assert!(
            (got - exact).abs() < 1e-6 * exact,
            "level {}: got {} vs exact-discrete {}",
            k,
            got,
            exact
        );
    }
    // Ground state also close to the continuum value E_1 = π²ħ²/(2mL²) ≈ 4.9348.
    let continuum_e1 = std::f64::consts::PI.powi(2) * hbar * hbar / (2.0 * mass * width * width);
    assert!(
        (res.eigenvalues[0] - continuum_e1).abs() / continuum_e1 < 0.01,
        "ground state {} vs continuum {}",
        res.eigenvalues[0],
        continuum_e1
    );
    // Eigenvalues must be ascending.
    assert!(res.eigenvalues[0] < res.eigenvalues[1]);
    assert!(res.eigenvalues[1] < res.eigenvalues[2]);
}

#[test]
fn logistic_growth_matches_analytic() {
    // N(t) = K / (1 + ((K-N0)/N0)·e^{-r t}).
    let lib = PhysicsSimulationLibrary::new();
    let (n0, r, k, total) = (1.0, 0.8, 100.0, 15.0);
    let res = lib.run_logistic_growth(n0, r, k, total, 60).unwrap();
    for (i, &t) in res.times.iter().enumerate() {
        let analytic = k / (1.0 + ((k - n0) / n0) * (-r * t).exp());
        assert!(
            (res.population[i] - analytic).abs() < 1e-4 * k.max(1.0),
            "t={}: got {} vs analytic {}",
            t,
            res.population[i],
            analytic
        );
    }
    // Approaches carrying capacity.
    assert!((res.population.last().unwrap() - k).abs() < 0.1);
}

#[test]
fn advection_diffusion_pure_diffusion_relaxes_and_conserves() {
    // c = 0 → pure diffusion on a periodic ring: total conserved, profile → mean.
    let lib = PhysicsSimulationLibrary::new();
    let n = 32;
    let dx = 1.0 / n as f64;
    let initial: Vec<f64> = (0..n)
        .map(|i| (2.0 * std::f64::consts::PI * i as f64 / n as f64).sin())
        .collect();
    let res = lib
        .run_advection_diffusion_1d(initial, 0.0, 0.05, dx, 1.0, 40)
        .unwrap();
    // Conservation of Σ u·dx.
    assert!(
        (res.final_total - res.initial_total).abs() < 1e-9,
        "total drifted: {} -> {}",
        res.initial_total,
        res.final_total
    );
    // Sine mean is ~0; after diffusion the field amplitude shrinks toward 0.
    let final_amp = res.final_field.iter().map(|v| v.abs()).fold(0.0, f64::max);
    assert!(final_amp < 0.5, "amplitude {} did not decay", final_amp);
}

#[test]
fn advection_diffusion_transports_and_conserves() {
    // Pure advection (c>0, alpha small) on a ring: total conserved, profile moves.
    let lib = PhysicsSimulationLibrary::new();
    let n = 64;
    let dx = 1.0 / n as f64;
    let initial: Vec<f64> = (0..n)
        .map(|i| (2.0 * std::f64::consts::PI * i as f64 / n as f64).sin())
        .collect();
    let res = lib
        .run_advection_diffusion_1d(initial.clone(), 1.0, 1e-4, dx, 0.25, 20)
        .unwrap();
    assert!((res.final_total - res.initial_total).abs() < 1e-9);
    // Field actually changed (transport occurred).
    let moved: f64 = res
        .final_field
        .iter()
        .zip(initial.iter())
        .map(|(a, b)| (a - b).abs())
        .sum();
    assert!(moved > 1e-3, "profile did not move under advection");
}

#[test]
fn test_distributed_simulation() {
    let mut library = PhysicsSimulationLibrary::new();
    library.initialize().unwrap();

    let config = SimulationConfig {
        simulation_id: "distributed_test".to_string(),
        simulation_type: SimulationType::CFD,
        domain_type: DomainType::ThreeDimensional,
        time_step: 0.001,
        total_time: 0.1,
        spatial_resolution: SpatialResolution {
            nx: 100,
            ny: Some(100),
            nz: Some(100),
            dx: 0.01,
            dy: Some(0.01),
            dz: Some(0.01),
        },
        numerical_method: NumericalMethod::FiniteVolume,
        parallel_config: ParallelConfig {
            num_threads: 8,
            num_processes: 4,
            domain_decomposition: DomainDecomposition::ThreeDimensional,
            load_balancing: LoadBalancing::LoadBased,
            communication_pattern: CommunicationPattern::Hybrid,
        },
    };

    let mut simulation = library.create_simulation(config).unwrap();

    let result = library.run_distributed_simulation(&mut simulation).unwrap();

    // Real per-node Burgers integration, aggregated. Assert the actual computed
    // behaviour: 3 merged fields, real iteration count, a finite computed residual.
    assert_eq!(result.result.len(), 3); // velocity + pressure + temperature, merged across nodes
    assert!(result.convergence_info.iterations > 0);
    assert!(result.convergence_info.residual_norm.is_finite());
    assert!(result
        .result
        .iter()
        .all(|f| f.data.iter().all(|v| v.is_finite())));
}

#[test]
fn test_performance_metrics() {
    let library = PhysicsSimulationLibrary::new();
    let metrics = library.get_performance_stats();

    assert_eq!(metrics.simulation_metrics.total_simulations, 0);
    assert_eq!(
        metrics
            .solver_metrics
            .linear_solver_metrics
            .average_iterations,
        0.0
    );
    assert_eq!(metrics.mesh_metrics.total_nodes, 0);
    assert_eq!(metrics.data_metrics.total_data_size, 0);
}

// ---- Feature 1: Boundary Conditions System ----

#[test]
fn test_boundary_conditions_dirichlet() {
    let mut bc = BoundaryConditions::new();
    bc.set_boundary("test_field", BoundaryType::Dirichlet, 42.0);

    let mut field = PhysicsField {
        field_id: "test_field".to_string(),
        field_type: FieldType::Scalar,
        dimensions: vec![5],
        data: vec![1.0, 2.0, 3.0, 4.0, 5.0],
        metadata: FieldMetadata {
            field_name: "Test".to_string(),
            physical_quantity: "Test".to_string(),
            units: "unit".to_string(),
            time_step: 0,
            iteration: 0,
        },
    };

    bc.apply_to_field(&mut field, 0.0);

    // Dirichlet: edge cells set to the boundary value.
    assert_eq!(field.data[0], 42.0);
    assert_eq!(field.data[4], 42.0);
    // Interior cells unchanged.
    assert_eq!(field.data[1], 2.0);
    assert_eq!(field.data[2], 3.0);
    assert_eq!(field.data[3], 4.0);
}

#[test]
fn test_boundary_conditions_periodic() {
    let mut bc = BoundaryConditions::new();
    bc.set_boundary("periodic_field", BoundaryType::Periodic, 0.0);

    let mut field = PhysicsField {
        field_id: "periodic_field".to_string(),
        field_type: FieldType::Scalar,
        dimensions: vec![5],
        data: vec![10.0, 1.0, 2.0, 3.0, 20.0],
        metadata: FieldMetadata {
            field_name: "Test".to_string(),
            physical_quantity: "Test".to_string(),
            units: "unit".to_string(),
            time_step: 0,
            iteration: 0,
        },
    };

    bc.apply_to_field(&mut field, 0.0);

    // Periodic: left edge copies inner neighbour of right edge (index n-2 = 3.0),
    // right edge copies inner neighbour of left edge (index 1 = 1.0).
    assert_eq!(field.data[0], 3.0);
    assert_eq!(field.data[4], 1.0);
}

#[test]
fn test_boundary_conditions_time_dependent() {
    let mut bc = BoundaryConditions::new();
    bc.set_time_dependent_boundary(
        "td_field",
        BoundaryType::Dirichlet,
        TimeFunction::Sinusoidal(10.0, 1.0, 0.0),
    );

    let mut field = PhysicsField {
        field_id: "td_field".to_string(),
        field_type: FieldType::Scalar,
        dimensions: vec![4],
        data: vec![0.0, 1.0, 2.0, 0.0],
        metadata: FieldMetadata {
            field_name: "Test".to_string(),
            physical_quantity: "Test".to_string(),
            units: "unit".to_string(),
            time_step: 0,
            iteration: 0,
        },
    };

    // At t = 0.25, sin(2*pi*1*0.25) = sin(pi/2) = 1.0 => value = 10.0
    bc.apply_to_field(&mut field, 0.25);
    assert!((field.data[0] - 10.0).abs() < 1e-10);
    assert!((field.data[3] - 10.0).abs() < 1e-10);
}

// ---- Feature 2: CFL Adaptive Time Stepping ----

#[test]
fn test_cfl_dt_computation_and_clamping() {
    let tsc = TimeStepControl::new();
    // CFL = 0.5, dx = 0.1, max_velocity = 10.0 => dt = 0.5 * 0.1 / 10.0 = 0.005
    let dt = tsc.compute_cfl_dt(10.0, 0.1);
    assert!((dt - 0.005).abs() < 1e-12);

    // Clamping to max_time_step (1.0): very small velocity => huge dt => clamped.
    let dt_max = tsc.compute_cfl_dt(1e-6, 0.1);
    assert!((dt_max - 1.0).abs() < 1e-12);

    // Clamping to min_time_step (1e-6): very large velocity => tiny dt => clamped.
    let dt_min = tsc.compute_cfl_dt(1e12, 0.1);
    assert!((dt_min - 1e-6).abs() < 1e-12);

    // Zero velocity => returns max_time_step (no advective limit).
    let dt_zero = tsc.compute_cfl_dt(0.0, 0.1);
    assert!((dt_zero - 1.0).abs() < 1e-12);
}

#[test]
fn test_cfl_update_dt_safety_and_limits() {
    let mut tsc = TimeStepControl::new();
    // Initial current_time_step = 0.001.
    // First call: CFL dt = 0.5 * 0.1 / 10 = 0.005, safety 0.9 => 0.0045.
    // But max_increase_factor = 2.0 caps the increase from 0.001 to 0.002.
    let dt1 = tsc.update_dt(10.0, 0.1);
    assert!((dt1 - 0.002).abs() < 1e-12);

    // Second call with much larger velocity: CFL dt = 0.5 * 0.1 / 1000 = 5e-5
    // safety => 4.5e-5, but max_decrease_factor = 0.5 => lower bound = 0.002 * 0.5 = 0.001
    // So dt is clamped to 0.001.
    let dt2 = tsc.update_dt(1000.0, 0.1);
    assert!((dt2 - 0.001).abs() < 1e-12);
}

#[test]
fn test_adaptive_step_cfl() {
    let mut integrator = TimeIntegrator::new();
    let field = PhysicsField {
        field_id: "vel".to_string(),
        field_type: FieldType::Vector,
        dimensions: vec![10],
        data: vec![5.0; 10],
        metadata: FieldMetadata {
            field_name: "Velocity".to_string(),
            physical_quantity: "Velocity".to_string(),
            units: "m/s".to_string(),
            time_step: 0,
            iteration: 0,
        },
    };

    // max_velocity = 5.0, dx ≈ 1/9, CFL = 0.5 => dt ≈ 0.5 * (1/9) / 5 ≈ 0.0111
    // safety 0.9 => ≈ 0.01
    let dt = integrator.adaptive_step(&field, 0.001);
    assert!(dt > 0.0);
    assert!(dt < 1.0); // within max bound
}

// ---- Feature: CFL Velocity Field Population ----

#[test]
fn test_cfl_velocity_field_max_velocity() {
    let mut cfl = CflCondition::new();
    // No velocity field yet: falls back to default sound_speed (343.0).
    assert!((cfl.max_velocity() - 343.0).abs() < 1e-12);

    cfl.set_velocity_field(vec![-1.0, 3.0, -7.0, 2.0]);
    // max absolute velocity = 7.0
    assert!((cfl.max_velocity() - 7.0).abs() < 1e-12);

    // Accessor returns the populated field.
    let field = cfl.get_velocity_field().unwrap();
    assert_eq!(field.len(), 4);
    assert_eq!(field[2], -7.0);
}

#[test]
fn test_cfl_max_velocity_sound_speed_fallback() {
    let mut cfl = CflCondition::new();
    cfl.set_sound_speed(500.0);
    // No velocity field => sound speed used.
    assert!((cfl.max_velocity() - 500.0).abs() < 1e-12);

    // Velocity field takes precedence over sound speed.
    cfl.set_velocity_field(vec![2.0, -4.0]);
    assert!((cfl.max_velocity() - 4.0).abs() < 1e-12);
}

#[test]
fn test_cfl_max_velocity_neither_set() {
    let mut cfl = CflCondition::new();
    cfl.sound_speed = None;
    assert!((cfl.max_velocity() - 0.0).abs() < 1e-12);
}

#[test]
fn test_compute_cfl_dt_from_field() {
    let mut tsc = TimeStepControl::new();
    tsc.cfl_condition.set_velocity_field(vec![1.0, -10.0, 5.0]);
    // max_velocity = 10.0, CFL = 0.5, dx = 0.1 => dt = 0.5 * 0.1 / 10 = 0.005
    let dt = tsc.compute_cfl_dt_from_field(0.1);
    assert!((dt - 0.005).abs() < 1e-12);
}

// ---- Feature: Diffusion Coefficient in CFL ----

#[test]
fn test_compute_diffusion_dt() {
    let tsc = TimeStepControl::new();
    // CFL = 0.5, dx = 0.1, diffusion_coeff = 1.0
    // dt_diff = 0.5 * 0.1^2 / (2 * 1.0) = 0.5 * 0.01 / 2 = 0.0025
    let dt = tsc.compute_diffusion_dt(1.0, 0.1);
    assert!((dt - 0.0025).abs() < 1e-12);

    // Zero diffusion coefficient => no diffusive limit => max_time_step.
    let dt_zero = tsc.compute_diffusion_dt(0.0, 0.1);
    assert!((dt_zero - 1.0).abs() < 1e-12);
}

#[test]
fn test_compute_diffusion_dt_clamping() {
    let tsc = TimeStepControl::new();
    // Very small diffusion => huge dt => clamped to max_time_step (1.0).
    let dt_max = tsc.compute_diffusion_dt(1e-9, 0.1);
    assert!((dt_max - 1.0).abs() < 1e-12);

    // Very large diffusion => tiny dt => clamped to min_time_step (1e-6).
    let dt_min = tsc.compute_diffusion_dt(1e9, 0.1);
    assert!((dt_min - 1e-6).abs() < 1e-12);
}

#[test]
fn test_compute_combined_dt_takes_minimum() {
    let tsc = TimeStepControl::new();
    // Advective: 0.5 * 0.1 / 10.0 = 0.005
    // Diffusive: 0.5 * 0.01 / 2.0 = 0.0025
    // Combined => min(0.005, 0.0025) = 0.0025
    let dt = tsc.compute_combined_dt(10.0, 1.0, 0.1);
    assert!((dt - 0.0025).abs() < 1e-12);

    // Swap so advective is more restrictive.
    // Advective: 0.5 * 0.1 / 100.0 = 0.0005
    // Diffusive: 0.5 * 0.01 / 2.0 = 0.0025
    // Combined => min(0.0005, 0.0025) = 0.0005
    let dt2 = tsc.compute_combined_dt(100.0, 1.0, 0.1);
    assert!((dt2 - 0.0005).abs() < 1e-12);
}

#[test]
fn test_cfl_set_diffusion_coefficient() {
    let mut cfl = CflCondition::new();
    assert!(cfl.diffusion_coefficient.is_none());
    cfl.set_diffusion_coefficient(2.5);
    assert_eq!(cfl.diffusion_coefficient, Some(2.5));
}

// ---- Feature: AdaptiveParameters Usage ----

#[test]
fn test_adaptive_parameters_with_values() {
    let params = AdaptiveParameters::with_values(1e-4, 0.5, 0.8);
    assert!((params.min_time_step - 1e-4).abs() < 1e-12);
    assert!((params.max_time_step - 0.5).abs() < 1e-12);
    assert!((params.safety_factor - 0.8).abs() < 1e-12);
    // Relative limits keep defaults.
    assert!((params.max_increase_factor - 2.0).abs() < 1e-12);
    assert!((params.max_decrease_factor - 0.5).abs() < 1e-12);
}

#[test]
fn test_adaptive_parameters_clamp_dt() {
    let params = AdaptiveParameters::with_values(1e-3, 1.0, 0.9);

    // Within bounds => unchanged.
    assert!((params.clamp_dt(0.5) - 0.5).abs() < 1e-12);

    // Below min => clamped to min.
    assert!((params.clamp_dt(1e-6) - 1e-3).abs() < 1e-12);

    // Above max => clamped to max.
    assert!((params.clamp_dt(10.0) - 1.0).abs() < 1e-12);
}

#[test]
fn test_adaptive_parameters_apply_safety_factor() {
    let params = AdaptiveParameters::with_values(1e-4, 1.0, 0.5);

    // 0.5 * 0.5 = 0.25, within bounds.
    assert!((params.apply_safety_factor(0.5) - 0.25).abs() < 1e-12);

    // 10.0 * 0.5 = 5.0, clamped to max 1.0.
    assert!((params.apply_safety_factor(10.0) - 1.0).abs() < 1e-12);

    // 1e-5 * 0.5 = 5e-6, clamped to min 1e-4.
    assert!((params.apply_safety_factor(1e-5) - 1e-4).abs() < 1e-12);
}

#[test]
fn test_update_dt_uses_adaptive_parameters_clamp_and_safety() {
    let mut tsc = TimeStepControl::new();
    tsc.adaptive_parameters = AdaptiveParameters::with_values(1e-6, 1.0, 0.9);
    tsc.current_time_step = 0.0; // bypass relative limiter on first call

    // CFL dt = 0.5 * 0.1 / 10 = 0.005, safety 0.9 => 0.0045, within bounds.
    let dt = tsc.update_dt(10.0, 0.1);
    assert!((dt - 0.0045).abs() < 1e-12);
    assert!((tsc.current_time_step - 0.0045).abs() < 1e-12);
}

// ---- Feature 3: Stencil Operators ----

#[test]
fn test_central_difference_stencil() {
    let mut ops = StencilOperators::new();
    ops.register_operator("central2", StencilOperators::central_difference_2nd_order());

    // f(x) = x^2, derivative f'(x) = 2x.
    // dx = 1.0, field = [0, 1, 4, 9, 16, 25] (x = 0..5)
    let field = vec![0.0_f64, 1.0, 4.0, 9.0, 16.0, 25.0];
    let dx = 1.0;

    // At index 2 (x=2): f'(2) = 4. Central diff = (f[3]-f[1])/(2*dx) = (9-1)/2 = 4.0
    let deriv = ops.apply_derivative("central2", &field, dx, 2).unwrap();
    assert!((deriv - 4.0).abs() < 1e-12);

    // At index 3 (x=3): f'(3) = 6. Central diff = (f[4]-f[2])/(2*dx) = (16-4)/2 = 6.0
    let deriv3 = ops.apply_derivative("central2", &field, dx, 3).unwrap();
    assert!((deriv3 - 6.0).abs() < 1e-12);
}

#[test]
fn test_forward_difference_stencil() {
    let mut ops = StencilOperators::new();
    ops.register_operator("forward1", StencilOperators::forward_difference_1st_order());

    // f(x) = x, derivative = 1 everywhere.
    let field = vec![0.0_f64, 1.0, 2.0, 3.0, 4.0];
    let dx = 1.0;

    let deriv = ops.apply_derivative("forward1", &field, dx, 0).unwrap();
    assert!((deriv - 1.0).abs() < 1e-12);
}

#[test]
fn test_stencil_out_of_bounds() {
    let mut ops = StencilOperators::new();
    ops.register_operator("central2", StencilOperators::central_difference_2nd_order());

    let field = vec![0.0_f64, 1.0, 2.0];
    // At index 0, central diff needs index -1 => out of bounds.
    let result = ops.apply_derivative("central2", &field, 1.0, 0);
    assert!(result.is_err());
}

// ---- Feature 4: ZNS/CSD Data Persistence ----

#[test]
fn test_store_and_retrieve_field_data() {
    let mut storage = PhysicsDataStorage::new();
    storage.initialize().unwrap();

    // Verify default backends were registered.
    assert!(storage.storage_backends.contains_key("zns"));
    assert!(storage.storage_backends.contains_key("csd"));

    let field = PhysicsField {
        field_id: "test_store_field".to_string(),
        field_type: FieldType::Scalar,
        dimensions: vec![4],
        data: vec![1.0, 2.0, 3.0, 4.0],
        metadata: FieldMetadata {
            field_name: "Test".to_string(),
            physical_quantity: "Test".to_string(),
            units: "unit".to_string(),
            time_step: 0,
            iteration: 0,
        },
    };

    // Store and retrieve.
    storage.store_field_data(&field).unwrap();
    let retrieved = storage.retrieve_field_data("test_store_field");
    assert!(retrieved.is_some());
    let data = retrieved.unwrap();
    assert_eq!(data, vec![1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn test_store_field_data_no_backends() {
    let mut storage = PhysicsDataStorage::new();
    // No initialize() call => no backends registered.
    let field = PhysicsField {
        field_id: "no_backend".to_string(),
        field_type: FieldType::Scalar,
        dimensions: vec![2],
        data: vec![1.0, 2.0],
        metadata: FieldMetadata {
            field_name: "Test".to_string(),
            physical_quantity: "Test".to_string(),
            units: "unit".to_string(),
            time_step: 0,
            iteration: 0,
        },
    };

    let result = storage.store_field_data(&field);
    assert!(result.is_err());
}

// ---- Feature 5: Wire MeshNetworkManager ----

#[test]
fn test_mesh_network_init_and_status() {
    let mut coordinator = MeshCoordinator::new();

    // Initialize the mesh network.
    let init_result = coordinator.initialize_mesh_network();
    assert!(init_result.is_ok());

    // Query status.
    let status = coordinator.get_mesh_status().unwrap();
    // After initialization the network has zero nodes (no hardware discovered),
    // but the call must succeed and return finite values.
    assert!(status.total_nodes < u32::MAX);
}

#[test]
fn test_mesh_distribute_task() {
    let mut coordinator = MeshCoordinator::new();
    coordinator.initialize_mesh_network().unwrap();

    let task_data = b"simulation_task_payload";
    let result = coordinator.distribute_simulation_task(task_data);
    assert!(result.is_ok());
}
