
use super::*;

#[test]
fn test_engineering_library_creation() {
    let mut library = EngineeringAnalysisLibrary::new();
    assert!(library.initialize().is_ok());
}

#[test]
fn fluid_analyzer_runs_real_cfd_not_notimplemented() {
    // Regression: FluidAnalyzer::analyze used to return NotImplemented even
    // though cfd::run_cfd was fully built and tested. It must now run the
    // solver and return finite fields (which also catches solver blow-up).
    let mut analyzer = FluidAnalyzer::new();
    let mut model = EngineeringModel::new();
    model.geometry.dimensions = vec![1.0, 1.0]; // [Lx, Ly]
    let mut mat = Material::new();
    mat.material_properties.density = 1.0; // water-like
    model.materials.insert("fluid".to_string(), mat);

    let result = analyzer
        .analyze(&model, AnalysisType::LinearStatic)
        .expect("CFD analyze should run the solver, not return NotImplemented");
    assert_eq!(result.displacement_field.len(), 32 * 32);
    assert_eq!(result.stress_field.len(), 32 * 32);
    assert!(
        result.displacement_field.iter().all(|x| x.is_finite()),
        "velocity field must be finite (no solver blow-up)"
    );
    assert!(result.stress_field.iter().all(|x| x.is_finite()));
}

#[test]
fn test_structural_analysis() {
    let mut library = EngineeringAnalysisLibrary::new();
    library.initialize().unwrap();

    // Real axial member: steel (E=200000, σ_yield=250), area = 1×1, length = 2, axial load 50.
    // ⇒ σ = F/A = 50,  FoS = σ_yield/σ = 5,  ε = σ/E,  δ = F·L/(A·E).
    let mut model = EngineeringModel::new();
    model.geometry.dimensions = vec![1.0, 1.0, 2.0];
    model.materials.insert("steel".to_string(), Material::new()); // yield 250, E 200000
    model.loads.push(Load {
        load_id: "F".to_string(),
        load_type: LoadType::Point,
        load_magnitude: 50.0,
        load_direction: vec![1.0, 0.0, 0.0],
        application_point: vec![0.0, 0.0, 0.0],
    });

    let result = library
        .perform_structural_analysis(model, AnalysisType::LinearStatic)
        .unwrap();
    let r = &result.result;
    // REAL computed values, not a fabricated 2.5 safety factor.
    assert!(
        (r.stress_field[0] - 50.0).abs() < 1e-9,
        "stress = {}",
        r.stress_field[0]
    );
    assert!(
        (r.safety_factor - 5.0).abs() < 1e-9,
        "FoS = {}",
        r.safety_factor
    );
    assert!((r.strain_field[0] - 50.0 / 200000.0).abs() < 1e-12);
    assert!((r.displacement_field[0] - 50.0 * 2.0 / (1.0 * 200000.0)).abs() < 1e-12);
    // A bigger load ⇒ smaller safety factor (monotonic, real physics).
    let mut m2 = EngineeringModel::new();
    m2.geometry.dimensions = vec![1.0, 1.0, 2.0];
    m2.materials.insert("steel".to_string(), Material::new());
    m2.loads.push(Load {
        load_id: "F2".to_string(),
        load_type: LoadType::Point,
        load_magnitude: 100.0,
        load_direction: vec![1.0, 0.0, 0.0],
        application_point: vec![0.0, 0.0, 0.0],
    });
    let fos2 = library
        .perform_structural_analysis(m2, AnalysisType::LinearStatic)
        .unwrap()
        .result
        .safety_factor;
    assert!(fos2 < r.safety_factor);
}

#[test]
fn test_mechanical_analysis() {
    let mut library = EngineeringAnalysisLibrary::new();
    library.initialize().unwrap();

    let model = EngineeringModel::new();
    // HONEST: mechanical FE analysis isn't implemented → NotImplemented, not a fake result.
    let result = library.perform_mechanical_analysis(model, AnalysisType::LinearDynamic);
    assert!(matches!(result, Err(EngineeringError::NotImplemented(_))));
}

#[test]
fn test_thermal_analysis() {
    let mut library = EngineeringAnalysisLibrary::new();
    library.initialize().unwrap();

    // REAL: 1-D steady conduction. A bar with k=50, length 2, ends held at
    // 100 K and 300 K — the facade returns a genuine linear temperature field
    // (proofs live in thermal_conduction.rs).
    let mut materials = std::collections::HashMap::new();
    materials.insert(
        "steel".to_string(),
        Material {
            material_id: "steel".to_string(),
            material_name: "steel".to_string(),
            material_properties: MaterialProperties {
                youngs_modulus: 200000.0,
                poissons_ratio: 0.3,
                density: 7850.0,
                thermal_expansion: 1.2e-5,
                thermal_conductivity: 50.0,
                specific_heat: 500.0,
                yield_strength: 250.0,
                ultimate_strength: 400.0,
            },
        },
    );
    let model = EngineeringModel {
        model_id: "bar".to_string(),
        model_name: "bar".to_string(),
        model_type: ModelType::Thermal,
        geometry: Geometry {
            geometry_type: GeometryType::Beam,
            dimensions: vec![0.1, 0.1, 2.0],
            features: Vec::new(),
        },
        materials,
        boundary_conditions: vec![
            BoundaryCondition {
                condition_id: "l".to_string(),
                condition_type: BoundaryConditionType::Temperature,
                condition_value: 100.0,
            },
            BoundaryCondition {
                condition_id: "r".to_string(),
                condition_type: BoundaryConditionType::Temperature,
                condition_value: 300.0,
            },
        ],
        loads: Vec::new(),
    };
    let result = library
        .perform_thermal_analysis(model, AnalysisType::Thermal)
        .unwrap();
    let t = &result.result.temperature_field;
    assert!(t.len() >= 2);
    assert!((t[0] - 100.0).abs() < 1e-6 && (t[t.len() - 1] - 300.0).abs() < 1e-6);
    assert!(result.convergence_info.converged);

    // A model with no thermal boundary conditions must be refused, not faked.
    let bare = EngineeringModel::new();
    assert!(matches!(
        library.perform_thermal_analysis(bare, AnalysisType::Thermal),
        Err(EngineeringError::InsufficientData(_))
    ));
}

#[test]
fn test_fluid_analysis() {
    let mut library = EngineeringAnalysisLibrary::new();
    library.initialize().unwrap();

    // Real CFD now runs through the library routing (cfd::run_cfd, LBM/D2Q9)
    // — a proper fluid model yields finite fields, not NotImplemented.
    let mut model = EngineeringModel::new();
    model.geometry.dimensions = vec![1.0, 1.0];
    let mut mat = Material::new();
    mat.material_properties.density = 1.0;
    model.materials.insert("fluid".to_string(), mat);
    let result = library
        .perform_fluid_analysis(model, AnalysisType::LinearStatic)
        .expect("real CFD solve");
    assert!(!result.result.displacement_field.is_empty());
    assert!(result
        .result
        .displacement_field
        .iter()
        .all(|x| x.is_finite()));

    // A model with no geometry dimensions must be refused, not faked.
    let bare = EngineeringModel::new();
    assert!(library
        .perform_fluid_analysis(bare, AnalysisType::LinearStatic)
        .is_err());
}

#[test]
fn test_reliability_analysis() {
    let mut library = EngineeringAnalysisLibrary::new();
    library.initialize().unwrap();

    // A bare EngineeringModel::new() has no materials and no loads, so the
    // real reliability analyzer refuses with InsufficientData rather than
    // fabricating a result.
    let bare = EngineeringModel::new();
    let result = library.perform_reliability_analysis(bare, AnalysisType::LinearStatic);
    assert!(matches!(result, Err(EngineeringError::InsufficientData(_))));

    // With a real material and load, the analyzer computes a genuine
    // reliability index from stress vs. yield strength.
    let mut materials = std::collections::HashMap::new();
    materials.insert(
        "steel".to_string(),
        Material {
            material_id: "steel".to_string(),
            material_name: "steel".to_string(),
            material_properties: MaterialProperties {
                youngs_modulus: 200_000.0,
                poissons_ratio: 0.3,
                density: 7850.0,
                thermal_expansion: 1.2e-5,
                thermal_conductivity: 50.0,
                specific_heat: 500.0,
                yield_strength: 250.0e6,
                ultimate_strength: 400.0e6,
            },
        },
    );
    let model = EngineeringModel {
        model_id: "rel_test".to_string(),
        model_name: "Reliability Test".to_string(),
        model_type: ModelType::Structural,
        geometry: Geometry {
            geometry_type: GeometryType::Beam,
            dimensions: vec![0.1, 0.1, 1.0],
            features: Vec::new(),
        },
        materials,
        boundary_conditions: Vec::new(),
        loads: vec![Load {
            load_id: "f1".to_string(),
            load_type: LoadType::Force,
            load_magnitude: 1000.0,
            load_direction: vec![0.0, 0.0, -1.0],
            application_point: vec![0.5, 0.0, 0.0],
        }],
    };
    let result = library
        .perform_reliability_analysis(model, AnalysisType::LinearStatic)
        .unwrap();
    assert!(
        result.result.reliability_index > 0.0,
        "safe model should have positive β"
    );
    assert!(
        result.result.failure_probability < 0.01,
        "safe model should have Pf < 1%"
    );
    assert!(result.convergence_info.converged);
}

#[test]
fn test_performance_metrics() {
    let library = EngineeringAnalysisLibrary::new();
    let metrics = library.get_performance_stats();

    assert_eq!(metrics.total_analyses, 0);
    assert_eq!(metrics.average_computation_time, 0.0);
    // Honest: per-analysis accuracy is not tracked by this summary, so it is not fabricated.
    assert!(metrics.average_accuracy.is_none());
}

#[test]
fn test_analysis_types() {
    let library = EngineeringAnalysisLibrary::new();
    let types = library.list_analysis_types();

    assert!(types.contains(&"LinearStatic".to_string()));
    assert!(types.contains(&"NonlinearStatic".to_string()));
    assert!(types.contains(&"LinearDynamic".to_string()));
}

#[test]
fn test_model_info() {
    let library = EngineeringAnalysisLibrary::new();
    let info = library.get_model_info("model_1");
    assert!(info.is_none());
}

// ─── Feature 1: Phase 2 dependency wiring ───────────────────────────────
//
// `ZnsZoneManager::new` requires a real ZNS block device, so the top-level
// `attach_dependencies` (which takes all four deps) is not exercised here.
// Instead the three in-memory libraries are attached to their sub-analyzers
// directly, proving the wiring compiles and stores the dependencies. The
// library must also still initialise with all deps = None.

#[test]
fn test_dependency_wiring_sub_analyzers() {
    let la = Arc::new(Mutex::new(LinearAlgebraLibrary::new()));
    let phys = Arc::new(Mutex::new(PhysicsSimulationLibrary::new()));
    let stat = Arc::new(Mutex::new(StatisticalComputingLibrary::new()));

    let mut lib = EngineeringAnalysisLibrary::new();
    // Defaults are None — initialisation must succeed without dependencies.
    assert!(lib.initialize().is_ok());

    // Attach the three in-memory libraries to their owning sub-analyzers.
    lib.structural_analyzer
        .attach_linear_algebra(Some(la.clone()));
    lib.mechanical_analyzer
        .attach_physics_simulation(Some(phys.clone()));
    lib.thermal_analyzer
        .attach_physics_simulation(Some(phys.clone()));
    lib.reliability_analyzer
        .attach_statistical_computing(Some(stat.clone()));

    // Re-initialise after attaching — still ok.
    assert!(lib.initialize().is_ok());
}

// ─── Feature 2: MeshGenerator registry ─────────────────────────────────

#[test]
fn test_mesh_generator_initialization_and_accessors() {
    let mut mesh = MeshGenerator::new();
    // Before init the registries are empty.
    assert!(mesh.list_mesh_types().is_empty());
    assert!(mesh.list_algorithms().is_empty());

    assert!(mesh.initialize().is_ok());

    // Standard mesh types are registered.
    let types = mesh.list_mesh_types();
    assert!(types.contains(&"triangular".to_string()));
    assert!(types.contains(&"quadrilateral".to_string()));
    assert!(types.contains(&"tetrahedral".to_string()));
    assert!(types.contains(&"hexahedral".to_string()));
    assert!(types.contains(&"prism".to_string()));
    assert!(types.contains(&"pyramid".to_string()));

    // Standard algorithms are registered.
    let algos = mesh.list_algorithms();
    assert!(algos.contains(&"delaunay".to_string()));
    assert!(algos.contains(&"advancing_front".to_string()));
    assert!(algos.contains(&"octree".to_string()));
    assert!(algos.contains(&"structured".to_string()));
    assert!(algos.contains(&"unstructured".to_string()));

    // Accessors return the right variants.
    assert_eq!(
        mesh.get_mesh_type("triangular"),
        Some(&MeshType::Triangular)
    );
    assert_eq!(
        mesh.get_mesh_type("hexahedral"),
        Some(&MeshType::Hexahedral)
    );
    assert!(matches!(
        mesh.get_algorithm("delaunay"),
        Some(a) if a.algorithm_type == MeshAlgorithmType::Delaunay
    ));
    assert!(mesh.get_mesh_type("nonexistent").is_none());
    assert!(mesh.get_algorithm("nonexistent").is_none());
}

// ─── Feature 3: ElementLibrary standard FEA elements ───────────────────

#[test]
fn test_element_library_initialization_and_accessors() {
    let mut lib = ElementLibrary::new();
    assert!(lib.list_elements().is_empty());

    assert!(lib.initialize().is_ok());

    let names = lib.list_elements();
    for expected in [
        "truss_2node",
        "beam_2node",
        "quad_4node",
        "hex_8node",
        "tet_4node",
        "shell_8node",
    ] {
        assert!(names.contains(&expected.to_string()), "missing {expected}");
    }

    // Truss: 2 nodes, 2 DOF each.
    let truss = lib.get_element("truss_2node").unwrap();
    assert_eq!(truss.element_type, ElementType::Truss);
    assert_eq!(truss.nodes.len(), 2);
    assert_eq!(truss.nodes[0].degrees_of_freedom.len(), 2);

    // Beam: 2 nodes, 3 DOF each.
    let beam = lib.get_element("beam_2node").unwrap();
    assert_eq!(beam.element_type, ElementType::Beam);
    assert_eq!(beam.nodes.len(), 2);
    assert_eq!(beam.nodes[0].degrees_of_freedom.len(), 3);

    // Quad shell: 4 nodes, 2 DOF each.
    let quad = lib.get_element("quad_4node").unwrap();
    assert_eq!(quad.element_type, ElementType::Shell);
    assert_eq!(quad.nodes.len(), 4);
    assert_eq!(quad.nodes[0].degrees_of_freedom.len(), 2);

    // Hex solid: 8 nodes, 3 DOF each.
    let hex = lib.get_element("hex_8node").unwrap();
    assert_eq!(hex.element_type, ElementType::Hexahedron);
    assert_eq!(hex.nodes.len(), 8);
    assert_eq!(hex.nodes[0].degrees_of_freedom.len(), 3);

    // Tet solid: 4 nodes, 3 DOF each.
    let tet = lib.get_element("tet_4node").unwrap();
    assert_eq!(tet.element_type, ElementType::Tetrahedron);
    assert_eq!(tet.nodes.len(), 4);
    assert_eq!(tet.nodes[0].degrees_of_freedom.len(), 3);

    // Shell: 8 nodes, 6 DOF each.
    let shell = lib.get_element("shell_8node").unwrap();
    assert_eq!(shell.element_type, ElementType::Shell);
    assert_eq!(shell.nodes.len(), 8);
    assert_eq!(shell.nodes[0].degrees_of_freedom.len(), 6);

    // Properties accessor.
    assert!(lib.get_properties("truss_2node").is_some());
    assert!(lib.get_properties("nonexistent").is_none());
    assert!(lib.get_element("nonexistent").is_none());
}

// ─── Feature 4: Monte Carlo reliability analysis ───────────────────────

#[test]
fn test_monte_carlo_run_simulation_statistics() {
    let mut mc = MonteCarlo::new();
    let mean = 100.0;
    let std_dev = 10.0;
    let n = 20000;
    let samples = mc.run_simulation(mean, std_dev, n);
    assert_eq!(samples.len(), n);
    assert_eq!(mc.simulation_results.len(), n);
    assert_eq!(mc.num_simulations, n as u32);

    // Sample mean should be close to the population mean (loose tolerance).
    let sample_mean: f64 = samples.iter().sum::<f64>() / n as f64;
    assert!(
        (sample_mean - mean).abs() < 1.0,
        "sample mean {sample_mean} too far from {mean}"
    );
    // Sample std-dev should be close to the population std-dev.
    let var: f64 = samples
        .iter()
        .map(|x| (x - sample_mean).powi(2))
        .sum::<f64>()
        / n as f64;
    let sample_std = var.sqrt();
    assert!(
        (sample_std - std_dev).abs() < 2.0,
        "sample std {sample_std} too far from {std_dev}"
    );
}

#[test]
fn test_monte_carlo_reliability_known_inputs() {
    // Capacity threshold = 100, load ~ N(100, 10). Roughly half the samples
    // fall below the threshold ⇒ Pf ≈ 0.5 ⇒ β ≈ 0.
    let mut analyzer = ReliabilityAnalyzer::new();
    let result = analyzer.analyze_monte_carlo(&[100.0], 100.0, 10.0).unwrap();

    assert_eq!(result.results_id, "monte_carlo");
    assert!(
        (result.failure_probability - 0.5).abs() < 0.05,
        "Pf {} should be ~0.5",
        result.failure_probability
    );
    assert!(
        result.reliability_index.abs() < 0.2,
        "β {} should be ~0",
        result.reliability_index
    );
}

#[test]
fn test_monte_carlo_reliability_high_reliability() {
    // g(x) = x − threshold, failure when x < threshold. With load ~ N(100, 10)
    // and threshold = 70, Pf = P(x < 70) = Φ((70−100)/10) = Φ(−3) ≈ 0.00135
    // ⇒ β ≈ 3.0.
    let mut analyzer = ReliabilityAnalyzer::new();
    let result = analyzer.analyze_monte_carlo(&[70.0], 100.0, 10.0).unwrap();

    // With 10k samples the estimate is noisy at Pf~0.001; allow a wide band.
    assert!(
        result.failure_probability < 0.01,
        "Pf {} should be small",
        result.failure_probability
    );
    assert!(
        result.reliability_index > 2.0,
        "β {} should be > 2",
        result.reliability_index
    );
}

#[test]
fn test_reliability_index_inverse_normal() {
    let analyzer = ReliabilityAnalyzer::new();
    // Φ⁻¹(0.5) = 0 ⇒ β = 0.
    assert!((analyzer.compute_reliability_index(0.5)).abs() < 1e-6);
    // Φ⁻¹(0.001) ≈ −3.09 ⇒ β ≈ 3.09.
    let beta = analyzer.compute_reliability_index(0.001);
    assert!((beta - 3.09).abs() < 0.05, "β {beta}");
}

#[test]
fn test_monte_carlo_empty_limit_state() {
    let mut analyzer = ReliabilityAnalyzer::new();
    assert!(matches!(
        analyzer.analyze_monte_carlo(&[], 100.0, 10.0),
        Err(EngineeringError::InsufficientData(_))
    ));
}

// ─── Feature 5: MechanicalAnalyzer kinematics & dynamics ───────────────

#[test]
fn test_kinematics_known_values() {
    let mut ma = MechanicalAnalyzer::new();
    // x₀ = 0, v₀ = 5, a = 2. At t = 0,1,2,3.
    let times = vec![0.0, 1.0, 2.0, 3.0];
    let r = ma.analyze_kinematics(0.0, 5.0, 2.0, &times).unwrap();

    assert_eq!(r.time_steps, times);
    // position(t) = 5t + t²
    assert!((r.positions[0] - 0.0).abs() < 1e-9);
    assert!((r.positions[1] - 6.0).abs() < 1e-9); // 5 + 1
    assert!((r.positions[2] - 14.0).abs() < 1e-9); // 10 + 4
    assert!((r.positions[3] - 24.0).abs() < 1e-9); // 15 + 9
                                                   // velocity(t) = 5 + 2t
    assert!((r.velocities[0] - 5.0).abs() < 1e-9);
    assert!((r.velocities[1] - 7.0).abs() < 1e-9);
    assert!((r.velocities[2] - 9.0).abs() < 1e-9);
    assert!((r.velocities[3] - 11.0).abs() < 1e-9);
    // acceleration is constant = 2.
    for &a in &r.accelerations {
        assert!((a - 2.0).abs() < 1e-9);
    }
}

#[test]
fn test_kinematics_empty_time_steps() {
    let mut ma = MechanicalAnalyzer::new();
    assert!(matches!(
        ma.analyze_kinematics(0.0, 0.0, 0.0, &[]),
        Err(EngineeringError::InsufficientData(_))
    ));
}

#[test]
fn test_dynamics_f_equals_ma_and_energy_conservation() {
    let mut ma = MechanicalAnalyzer::new();
    // m = 2, F = 6 ⇒ a = 3. v₀ = 0.
    let times = vec![0.0, 1.0, 2.0, 3.0];
    let r = ma.analyze_dynamics(2.0, 6.0, 0.0, &times).unwrap();

    // F = ma ⇒ a = F/m = 3.
    for &a in &r.accelerations {
        assert!((a - 3.0).abs() < 1e-9, "a = {a}");
    }
    // velocity(t) = 3t
    assert!((r.velocities[1] - 3.0).abs() < 1e-9);
    assert!((r.velocities[2] - 6.0).abs() < 1e-9);
    assert!((r.velocities[3] - 9.0).abs() < 1e-9);
    // position(t) = 1.5·t²
    assert!((r.positions[1] - 1.5).abs() < 1e-9);
    assert!((r.positions[2] - 6.0).abs() < 1e-9);
    assert!((r.positions[3] - 13.5).abs() < 1e-9);

    // Energy conservation: with PE = −F·x, KE + PE = ½·m·v₀² = 0 (v₀ = 0).
    assert!(
        r.total_energy.abs() < 1e-6,
        "total energy {} should be ~0 (conserved)",
        r.total_energy
    );
    // And the identity total = KE + PE holds.
    assert!((r.total_energy - (r.kinetic_energy + r.potential_energy)).abs() < 1e-9);

    // Cross-check at every step: ½·m·v² − F·x is constant.
    let conserved = 0.0; // ½·m·v₀²
    for i in 0..times.len() {
        let ke = 0.5 * 2.0 * r.velocities[i].powi(2);
        let pe = -6.0 * r.positions[i];
        assert!(
            (ke + pe - conserved).abs() < 1e-6,
            "energy not conserved at step {i}: {}",
            ke + pe
        );
    }
}

#[test]
fn test_dynamics_nonpositive_mass() {
    let mut ma = MechanicalAnalyzer::new();
    assert!(matches!(
        ma.analyze_dynamics(0.0, 10.0, 0.0, &[1.0]),
        Err(EngineeringError::ValidationError(_))
    ));
    assert!(matches!(
        ma.analyze_dynamics(-1.0, 10.0, 0.0, &[1.0]),
        Err(EngineeringError::ValidationError(_))
    ));
}

// ─── Feature 6: General reliability analysis (Monte Carlo) ─────────────

fn components(reliabilities: &[f64]) -> Vec<ComponentReliability> {
    reliabilities
        .iter()
        .enumerate()
        .map(|(i, &r)| ComponentReliability::new(format!("c{}", i + 1), 1.0 - r, 1000.0))
        .collect()
}

#[test]
fn test_series_system() {
    // 3 components in series, each 0.9 reliability => 0.9^3 = 0.729.
    let analyzer = ReliabilityAnalyzer::new();
    let config = ReliabilityConfig::new(SystemModel::Series, components(&[0.9, 0.9, 0.9]));
    let result = analyzer.analyze_reliability(&config).unwrap();
    assert!(
        (result.system_reliability - 0.729).abs() < 0.02,
        "series reliability {} should be ~0.729",
        result.system_reliability
    );
    assert!(
        result.confidence_interval.0 <= result.system_reliability
            && result.system_reliability <= result.confidence_interval.1,
        "point estimate must lie within CI {:?}",
        result.confidence_interval
    );
}

#[test]
fn test_parallel_system() {
    // 3 components in parallel, each 0.5 => 1 - 0.5^3 = 0.875.
    let analyzer = ReliabilityAnalyzer::new();
    let config = ReliabilityConfig::new(SystemModel::Parallel, components(&[0.5, 0.5, 0.5]));
    let result = analyzer.analyze_reliability(&config).unwrap();
    assert!(
        (result.system_reliability - 0.875).abs() < 0.02,
        "parallel reliability {} should be ~0.875",
        result.system_reliability
    );
}

#[test]
fn test_k_out_of_n() {
    // 2 out of 3, each 0.8 => P(>=2 of 3) = 0.512 + 0.384 = 0.896.
    let analyzer = ReliabilityAnalyzer::new();
    let config = ReliabilityConfig::new(
        SystemModel::KOutOfN { k: 2, n: 3 },
        components(&[0.8, 0.8, 0.8]),
    );
    let result = analyzer.analyze_reliability(&config).unwrap();
    assert!(
        (result.system_reliability - 0.896).abs() < 0.02,
        "k-out-of-n reliability {} should be ~0.896",
        result.system_reliability
    );
}

#[test]
fn test_perfect_components() {
    // All 1.0 reliability => system 1.0 (series and parallel).
    let analyzer = ReliabilityAnalyzer::new();
    let series = ReliabilityConfig::new(SystemModel::Series, components(&[1.0, 1.0, 1.0]));
    let r = analyzer.analyze_reliability(&series).unwrap();
    assert!(
        (r.system_reliability - 1.0).abs() < 1e-9,
        "perfect series reliability {} should be 1.0",
        r.system_reliability
    );
    assert!(r.failure_rate.abs() < 1e-9);
    assert!(r.mtbf.is_infinite());

    let parallel = ReliabilityConfig::new(SystemModel::Parallel, components(&[1.0, 1.0, 1.0]));
    let r = analyzer.analyze_reliability(&parallel).unwrap();
    assert!(
        (r.system_reliability - 1.0).abs() < 1e-9,
        "perfect parallel reliability {} should be 1.0",
        r.system_reliability
    );
}

#[test]
fn test_failed_components() {
    // All 0.0 reliability => system 0.0 (series and parallel).
    let analyzer = ReliabilityAnalyzer::new();
    let series = ReliabilityConfig::new(SystemModel::Series, components(&[0.0, 0.0, 0.0]));
    let r = analyzer.analyze_reliability(&series).unwrap();
    assert!(
        r.system_reliability.abs() < 1e-9,
        "failed series reliability {} should be 0.0",
        r.system_reliability
    );
    assert!((r.failure_rate - 1.0).abs() < 1e-9);

    let parallel = ReliabilityConfig::new(SystemModel::Parallel, components(&[0.0, 0.0, 0.0]));
    let r = analyzer.analyze_reliability(&parallel).unwrap();
    assert!(
        r.system_reliability.abs() < 1e-9,
        "failed parallel reliability {} should be 0.0",
        r.system_reliability
    );
}

#[test]
fn test_component_importance() {
    // Series system: Birnbaum importance of component i = product of the
    // other components' reliabilities. With reliabilities [0.9, 0.8, 0.7]:
    //   I(c1) = 0.8*0.7 = 0.56
    //   I(c2) = 0.9*0.7 = 0.63
    //   I(c3) = 0.9*0.8 = 0.72
    let analyzer = ReliabilityAnalyzer::new();
    let config = ReliabilityConfig::new(SystemModel::Series, components(&[0.9, 0.8, 0.7]));
    let result = analyzer.analyze_reliability(&config).unwrap();
    assert_eq!(result.component_importance.len(), 3);
    let i1 = *result.component_importance.get("c1").unwrap();
    let i2 = *result.component_importance.get("c2").unwrap();
    let i3 = *result.component_importance.get("c3").unwrap();
    assert!((i1 - 0.56).abs() < 1e-9, "I(c1) = {i1}");
    assert!((i2 - 0.63).abs() < 1e-9, "I(c2) = {i2}");
    assert!((i3 - 0.72).abs() < 1e-9, "I(c3) = {i3}");
    // Importance values are non-negative and bounded by 1.
    for &v in result.component_importance.values() {
        assert!((0.0..=1.0).contains(&v), "importance {v} out of [0,1]");
    }
}

#[test]
fn test_confidence_interval() {
    // The 95% CI must contain the point estimate and be a valid interval.
    let analyzer = ReliabilityAnalyzer::new();
    let config = ReliabilityConfig::new(SystemModel::Series, components(&[0.9, 0.9, 0.9]));
    let result = analyzer.analyze_reliability(&config).unwrap();
    let (lo, hi) = result.confidence_interval;
    assert!(lo <= hi, "CI lower {lo} > upper {hi}");
    assert!(
        lo <= result.system_reliability && result.system_reliability <= hi,
        "point estimate {} outside CI [{lo}, {hi}]",
        result.system_reliability
    );
    assert!(lo >= 0.0 && hi <= 1.0, "CI [{lo}, {hi}] out of [0,1]");
    // With 10k samples the CI half-width for p~0.73 is ~0.017.
    let half = (hi - lo) / 2.0;
    assert!(
        half > 0.0 && half < 0.05,
        "CI half-width {half} unreasonable"
    );
}

#[test]
fn test_reliability_analysis_validation() {
    let analyzer = ReliabilityAnalyzer::new();
    // Empty components.
    let cfg = ReliabilityConfig::new(SystemModel::Series, vec![]);
    assert!(matches!(
        analyzer.analyze_reliability(&cfg),
        Err(EngineeringError::InsufficientData(_))
    ));
    // Zero simulations.
    let mut cfg = ReliabilityConfig::new(SystemModel::Series, components(&[0.9]));
    cfg.num_simulations = 0;
    assert!(matches!(
        analyzer.analyze_reliability(&cfg),
        Err(EngineeringError::InsufficientData(_))
    ));
    // failure_probability out of range.
    let cfg = ReliabilityConfig::new(
        SystemModel::Series,
        vec![ComponentReliability::new("x", 1.5, 1000.0)],
    );
    assert!(matches!(
        analyzer.analyze_reliability(&cfg),
        Err(EngineeringError::ValidationError(_))
    ));
    // KOutOfN.n mismatch.
    let cfg = ReliabilityConfig::new(SystemModel::KOutOfN { k: 2, n: 3 }, components(&[0.8, 0.8]));
    assert!(matches!(
        analyzer.analyze_reliability(&cfg),
        Err(EngineeringError::ValidationError(_))
    ));
}

// ── ReliabilityAnalyzer::analyze (model-based) tests ──────────────────

fn reliability_model(force: f64, yield_strength: f64, ultimate_strength: f64) -> EngineeringModel {
    let mut materials = HashMap::new();
    materials.insert(
        "steel".to_string(),
        Material {
            material_id: "steel".to_string(),
            material_name: "steel".to_string(),
            material_properties: MaterialProperties {
                youngs_modulus: 200_000.0,
                poissons_ratio: 0.3,
                density: 7850.0,
                thermal_expansion: 1.2e-5,
                thermal_conductivity: 50.0,
                specific_heat: 500.0,
                yield_strength,
                ultimate_strength,
            },
        },
    );
    EngineeringModel {
        model_id: "rel_model".to_string(),
        model_name: "Reliability Test".to_string(),
        model_type: ModelType::Structural,
        geometry: Geometry {
            geometry_type: GeometryType::Beam,
            dimensions: vec![0.1, 0.1, 1.0],
            features: Vec::new(),
        },
        materials,
        boundary_conditions: Vec::new(),
        loads: vec![Load {
            load_id: "f1".to_string(),
            load_type: LoadType::Force,
            load_magnitude: force,
            load_direction: vec![0.0, 0.0, -1.0],
            application_point: vec![0.5, 0.0, 0.0],
        }],
    }
}

#[test]
fn reliability_analyze_safe_model_positive_beta() {
    // Force = 1000 N, area ≈ 0.01 m², stress = 100 kPa.
    // Yield = 250 MPa → SF = 2500. Very safe → β >> 0, Pf ≈ 0.
    let mut analyzer = ReliabilityAnalyzer::new();
    let model = reliability_model(1000.0, 250.0e6, 400.0e6);
    let result = analyzer
        .analyze(&model, AnalysisType::LinearStatic)
        .unwrap();
    assert!(
        result.reliability_index > 0.0,
        "safe model should have positive β"
    );
    assert!(
        result.failure_probability < 0.01,
        "safe model should have Pf < 1%"
    );
    assert!(result.mean_time_to_failure > 0.0);
    assert!(result.maintenance_interval > 0);
}

#[test]
fn reliability_analyze_yield_exceeded_negative_beta() {
    // Force = 1e9 N, area ≈ 0.01, stress = 1e11 Pa = 100 GPa.
    // Yield = 250 MPa → SF = 0.0025. Yield exceeded → β < 0, Pf > 0.5.
    let mut analyzer = ReliabilityAnalyzer::new();
    let model = reliability_model(1e9, 250.0e6, 400.0e6);
    let result = analyzer
        .analyze(&model, AnalysisType::LinearStatic)
        .unwrap();
    assert!(
        result.reliability_index < 0.0,
        "yield-exceeded model should have negative β"
    );
    assert!(
        result.failure_probability > 0.5,
        "yield-exceeded model should have Pf > 50%"
    );
}

#[test]
fn reliability_analyze_no_material_errors() {
    let mut analyzer = ReliabilityAnalyzer::new();
    let model = EngineeringModel {
        model_id: "empty".to_string(),
        model_name: "Empty".to_string(),
        model_type: ModelType::Structural,
        geometry: Geometry::new(),
        materials: HashMap::new(),
        boundary_conditions: Vec::new(),
        loads: Vec::new(),
    };
    let err = analyzer
        .analyze(&model, AnalysisType::LinearStatic)
        .unwrap_err();
    assert!(matches!(err, EngineeringError::InsufficientData(_)));
}

#[test]
fn reliability_analyze_no_loads_errors() {
    let mut analyzer = ReliabilityAnalyzer::new();
    let mut model = reliability_model(1000.0, 250.0e6, 400.0e6);
    model.loads.clear();
    let err = analyzer
        .analyze(&model, AnalysisType::LinearStatic)
        .unwrap_err();
    assert!(matches!(err, EngineeringError::InsufficientData(_)));
}

#[test]
fn reliability_analyze_results_id_contains_model_id() {
    let mut analyzer = ReliabilityAnalyzer::new();
    let model = reliability_model(1000.0, 250.0e6, 400.0e6);
    let result = analyzer
        .analyze(&model, AnalysisType::LinearStatic)
        .unwrap();
    assert!(result.results_id.contains("rel_model"));
}

// ---- Modal / free-vibration eigenproblem (wired to symmetric_eigen) --------

#[test]
fn modal_sdof_natural_frequency() {
    // Single DOF: k = 100, m = 1 ⇒ ω = √(k/m) = 10 rad/s.
    let mut modal = ModalAnalysis::new();
    let modes = modal.analyze_modal(&[100.0], &[1.0], 1).unwrap();
    assert_eq!(modes.len(), 1);
    assert!(
        (modes[0].natural_frequency - 10.0).abs() < 1e-9,
        "ω = {}",
        modes[0].natural_frequency
    );
    assert_eq!(modes[0].mode_number, 1);
}

#[test]
fn modal_two_dof_known_eigenvalues() {
    // K = [[2,-1],[-1,2]], M = I. Eigenvalues of K are 1 and 3
    // ⇒ ω = {1, √3}. Mode shapes: [1,1] (in-phase) and [1,-1] (out-of-phase).
    let mut modal = ModalAnalysis::new();
    let k = [2.0, -1.0, -1.0, 2.0];
    let m = [1.0, 1.0];
    let modes = modal.analyze_modal(&k, &m, 2).unwrap();
    assert_eq!(modes.len(), 2);
    assert!(
        (modes[0].natural_frequency - 1.0).abs() < 1e-9,
        "ω1 = {}",
        modes[0].natural_frequency
    );
    assert!(
        (modes[1].natural_frequency - 3.0_f64.sqrt()).abs() < 1e-9,
        "ω2 = {}",
        modes[1].natural_frequency
    );
    // First mode: components equal (ratio +1). Second: opposite (ratio -1).
    let r0 = modes[0].mode_shape_vector[0] / modes[0].mode_shape_vector[1];
    let r1 = modes[1].mode_shape_vector[0] / modes[1].mode_shape_vector[1];
    assert!((r0 - 1.0).abs() < 1e-6, "mode1 ratio = {}", r0);
    assert!((r1 + 1.0).abs() < 1e-6, "mode2 ratio = {}", r1);
}

#[test]
fn free_vibration_matches_modal() {
    let mut vib = VibrationAnalysis::new();
    let k = [2.0, -1.0, -1.0, 2.0];
    let m = [1.0, 1.0];
    let fv = vib.analyze_free(&k, &m, 2).unwrap();
    assert_eq!(fv.natural_frequencies.len(), 2);
    assert!((fv.natural_frequencies[0] - 1.0).abs() < 1e-9);
    assert!((fv.natural_frequencies[1] - 3.0_f64.sqrt()).abs() < 1e-9);
    assert_eq!(fv.damping_ratios, vec![0.0, 0.0]);
}

#[test]
fn modal_rejects_bad_dimensions() {
    let mut modal = ModalAnalysis::new();
    // 2 DOFs claimed, but stiffness has only 1 entry.
    assert!(matches!(
        modal.analyze_modal(&[1.0], &[1.0, 1.0], 2),
        Err(EngineeringError::ValidationError(_))
    ));
    // Non-positive mass.
    assert!(matches!(
        modal.analyze_modal(&[1.0], &[0.0], 1),
        Err(EngineeringError::ValidationError(_))
    ));
}

#[test]
fn sdof_natural_frequency_helper() {
    let vib = VibrationAnalysis::new();
    assert!((vib.natural_frequency_sdof(400.0, 4.0).unwrap() - 10.0).abs() < 1e-12);
    assert!(matches!(
        vib.natural_frequency_sdof(100.0, 0.0),
        Err(EngineeringError::ValidationError(_))
    ));
}

// ---- Forced harmonic response (closed-form FRF) ---------------------------

#[test]
fn forced_harmonic_response_known_values() {
    // m=1, c=2, k=100, F0=10.
    let mut vib = VibrationAnalysis::new();
    // ω=0 (static): X = F0/k = 0.1, phase 0.
    // ω=10 (=ωn): denom = √(0 + (2·10)²) = 20 ⇒ X = 0.5, phase = π/2.
    let fv = vib
        .analyze_harmonic_sdof(1.0, 2.0, 100.0, 10.0, &[0.0, 10.0])
        .unwrap();
    assert!((fv.response_amplitudes[0] - 0.1).abs() < 1e-12);
    assert!((fv.phase_angles[0] - 0.0).abs() < 1e-12);
    assert!(
        (fv.response_amplitudes[1] - 0.5).abs() < 1e-12,
        "resonant X = {}",
        fv.response_amplitudes[1]
    );
    assert!(
        (fv.phase_angles[1] - std::f64::consts::FRAC_PI_2).abs() < 1e-12,
        "resonant phase = {}",
        fv.phase_angles[1]
    );
}

// ---- Euler buckling (closed form) ----------------------------------------

#[test]
fn euler_buckling_known_critical_load() {
    // E = 200e9 Pa, I = 1e-6 m⁴, L = 2 m, K = 1 (pinned–pinned).
    // P_cr = π²·E·I / (K·L)² = π²·200e9·1e-6 / 4.
    let mut buck = BucklingAnalysis::new();
    let eb = buck.analyze_euler(200.0e9, 1.0e-6, 2.0, 1.0, 3).unwrap();
    let expected = std::f64::consts::PI.powi(2) * 200.0e9 * 1.0e-6 / 4.0;
    assert!(
        (eb.critical_loads[0] - expected).abs() < 1e-3,
        "P_cr = {} (expected {})",
        eb.critical_loads[0],
        expected
    );
    // Higher modes scale as n².
    assert!((eb.critical_loads[1] - 4.0 * expected).abs() < 1e-3);
    assert!((eb.critical_loads[2] - 9.0 * expected).abs() < 1e-3);
    // Mode-shape endpoints of a half-sine are ~0.
    assert!(eb.buckling_modes[0].mode_shape.first().unwrap().abs() < 1e-9);
    assert!(eb.buckling_modes[0].mode_shape.last().unwrap().abs() < 1e-9);
}

#[test]
fn euler_buckling_from_model() {
    // b = h = 0.1 m, L = 2 m, steel E = 200000 (default MaterialProperties).
    // I = 0.1·0.1³/12 = 8.3333e-6 m⁴, P_cr = π²·E·I / L².
    let mut model = EngineeringModel::new();
    model.geometry.dimensions = vec![0.1, 0.1, 2.0];
    model.materials.insert("steel".to_string(), Material::new());
    let mut buck = BucklingAnalysis::new();
    let eb = buck.analyze_from_model(&model, 1).unwrap();
    let i_weak = 0.1 * 0.1_f64.powi(3) / 12.0;
    let expected = std::f64::consts::PI.powi(2) * 200000.0 * i_weak / 4.0;
    assert!(
        (eb.critical_loads[0] - expected).abs() < 1e-6,
        "P_cr = {} (expected {})",
        eb.critical_loads[0],
        expected
    );
}

#[test]
fn euler_buckling_rejects_bad_inputs() {
    let mut buck = BucklingAnalysis::new();
    assert!(matches!(
        buck.analyze_euler(0.0, 1.0e-6, 2.0, 1.0, 1),
        Err(EngineeringError::InsufficientData(_))
    ));
    assert!(matches!(
        buck.analyze_euler(200.0e9, 1.0e-6, 0.0, 1.0, 1),
        Err(EngineeringError::ValidationError(_))
    ));
}

// ---- Structural AnalysisResults facade dispatch --------------------------

#[test]
fn structural_buckling_dispatch_load_factor() {
    // b=h=0.1, L=2, steel; axial load 1000 N.
    // λ = P_cr / P_applied.
    let mut library = EngineeringAnalysisLibrary::new();
    library.initialize().unwrap();
    let mut model = EngineeringModel::new();
    model.geometry.dimensions = vec![0.1, 0.1, 2.0];
    model.materials.insert("steel".to_string(), Material::new());
    model.loads.push(Load {
        load_id: "P".to_string(),
        load_type: LoadType::Force,
        load_magnitude: 1000.0,
        load_direction: vec![1.0, 0.0, 0.0],
        application_point: vec![0.0, 0.0, 0.0],
    });
    let res = library
        .perform_structural_analysis(model, AnalysisType::Buckling)
        .unwrap();
    let i_weak = 0.1 * 0.1_f64.powi(3) / 12.0;
    let p_cr = std::f64::consts::PI.powi(2) * 200000.0 * i_weak / 4.0;
    let expected_lambda = p_cr / 1000.0;
    assert!(
        (res.result.safety_factor - expected_lambda).abs() < 1e-6,
        "λ = {} (expected {})",
        res.result.safety_factor,
        expected_lambda
    );
}

#[test]
fn structural_vibration_facade_is_honest_not_implemented() {
    // Modal results don't fit the scalar-field AnalysisResults shape — the
    // facade returns an honest NotImplemented pointing to the real method.
    let mut library = EngineeringAnalysisLibrary::new();
    library.initialize().unwrap();
    let mut model = EngineeringModel::new();
    model.geometry.dimensions = vec![1.0, 1.0, 2.0];
    model.materials.insert("steel".to_string(), Material::new());
    model.loads.push(Load {
        load_id: "F".to_string(),
        load_type: LoadType::Force,
        load_magnitude: 50.0,
        load_direction: vec![1.0, 0.0, 0.0],
        application_point: vec![0.0, 0.0, 0.0],
    });
    let res = library.perform_structural_analysis(model, AnalysisType::Vibration);
    assert!(matches!(res, Err(EngineeringError::NotImplemented(_))));
}

// ---- FE-backed facade dispatch (real assembly + solve) -------------------

fn prismatic_model(load: f64) -> EngineeringModel {
    // b=h=0.1 (area 0.01), L=2, steel (E=200000, rho=7850, yield 250).
    let mut model = EngineeringModel::new();
    model.geometry.dimensions = vec![0.1, 0.1, 2.0];
    model.materials.insert("steel".to_string(), Material::new());
    model.loads.push(Load {
        load_id: "F".to_string(),
        load_type: LoadType::Force,
        load_magnitude: load,
        load_direction: vec![1.0, 0.0, 0.0],
        application_point: vec![0.0, 0.0, 0.0],
    });
    model
}

#[test]
fn structural_linear_dynamic_facade_dynamic_amplification() {
    // Suddenly-applied constant axial load ⇒ undamped SDOF peak = 2x static (DAF=2).
    let mut library = EngineeringAnalysisLibrary::new();
    library.initialize().unwrap();
    let stat = library
        .perform_structural_analysis(prismatic_model(200.0), AnalysisType::LinearStatic)
        .unwrap()
        .result
        .displacement_field[0];
    let dyn_peak = library
        .perform_structural_analysis(prismatic_model(200.0), AnalysisType::LinearDynamic)
        .unwrap()
        .result
        .displacement_field[0];
    // static axial u = F*L/(A*E) = 200*2/(0.01*200000) = 0.2.
    assert!((stat - 0.2).abs() < 1e-9, "static = {stat}");
    let daf = dyn_peak / stat;
    assert!(
        (daf - 2.0).abs() < 0.02,
        "dynamic amplification factor = {daf} (expected ~2)"
    );
}

#[test]
fn structural_nonlinear_static_facade_stiffens() {
    // Geometric-nonlinear bar: real Newton–Raphson solve, tip disp below the
    // linear estimate (Green-strain stiffening) but finite and positive.
    let mut library = EngineeringAnalysisLibrary::new();
    library.initialize().unwrap();
    let res = library
        .perform_structural_analysis(prismatic_model(200.0), AnalysisType::NonlinearStatic)
        .unwrap()
        .result;
    let u = res.displacement_field[0];
    let lin = 0.2; // linear axial estimate
    assert!(
        u.is_finite() && u > 0.0 && u < lin,
        "nonlinear u = {u} (linear {lin})"
    );
    // Green-strain stiffening is real but modest for this load (u/L ≈ 0.09).
    assert!(
        u > 0.15,
        "stiffening should be modest for a mild load: u = {u}"
    );
    assert!(res.safety_factor.is_finite() && res.safety_factor > 0.0);
}

#[test]
fn structural_nonlinear_dynamic_facade_runs_and_amplifies() {
    // Newmark + inner Newton: real transient, peak exceeds the nonlinear static tip.
    let mut library = EngineeringAnalysisLibrary::new();
    library.initialize().unwrap();
    let stat = library
        .perform_structural_analysis(prismatic_model(200.0), AnalysisType::NonlinearStatic)
        .unwrap()
        .result
        .displacement_field[0];
    let dyn_peak = library
        .perform_structural_analysis(prismatic_model(200.0), AnalysisType::NonlinearDynamic)
        .unwrap()
        .result
        .displacement_field[0];
    assert!(
        dyn_peak.is_finite() && dyn_peak > stat,
        "peak {dyn_peak} vs static {stat}"
    );
    assert!(
        dyn_peak < 2.1 * stat,
        "nonlinear DAF should not exceed ~2: {}",
        dyn_peak / stat
    );
}
