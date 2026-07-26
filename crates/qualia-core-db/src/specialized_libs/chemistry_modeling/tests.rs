
use super::*;

#[test]
fn test_chemistry_library_creation() {
    let mut library = ChemistryModelingLibrary::new();
    assert!(library.initialize().is_ok());
}

#[test]
fn test_molecular_dynamics() {
    let mut library = ChemistryModelingLibrary::new();
    library.initialize().unwrap();

    // REAL: a small argon cluster, integrated with velocity-Verlet under a
    // Lennard-Jones force field. The atoms actually move and energy is
    // conserved — see molecular_dynamics.rs for the gradient/conservation
    // proofs. Here we check the facade returns a real, converged trajectory.
    let mut config = SimulationConfig::new();
    config.time_step = 0.001;
    config.total_time = 2.0;
    config.temperature = 120.0;
    let m = 39.948;
    let molecule = Molecule {
        molecule_id: "ar4".to_string(),
        formula: "Ar4".to_string(),
        atoms: vec![
            Atom {
                atom_id: "a".into(),
                element: "Ar".into(),
                atomic_number: 18,
                mass: m,
                charge: 0.0,
                coordinates: vec![0.0, 0.0, 0.0],
            },
            Atom {
                atom_id: "b".into(),
                element: "Ar".into(),
                atomic_number: 18,
                mass: m,
                charge: 0.0,
                coordinates: vec![3.9, 0.0, 0.0],
            },
            Atom {
                atom_id: "c".into(),
                element: "Ar".into(),
                atomic_number: 18,
                mass: m,
                charge: 0.0,
                coordinates: vec![0.0, 3.9, 0.0],
            },
            Atom {
                atom_id: "d".into(),
                element: "Ar".into(),
                atomic_number: 18,
                mass: m,
                charge: 0.0,
                coordinates: vec![3.9, 3.9, 0.3],
            },
        ],
        bonds: Vec::new(),
        coordinates: Vec::new(),
        properties: MolecularProperties::new(),
    };

    let result = library.run_molecular_dynamics(config, molecule).unwrap();
    assert!(
        result.convergence_info.converged,
        "energy not conserved: drift {}",
        result.convergence_info.final_error
    );
    assert!(result.result.frames.len() >= 2);

    // An empty molecule must be refused, never faked.
    let empty = Molecule {
        atoms: Vec::new(),
        ..Molecule::new()
    };
    assert!(matches!(
        library.run_molecular_dynamics(SimulationConfig::new(), empty),
        Err(ChemistryError::InsufficientData(_))
    ));
}

#[test]
fn test_quantum_properties() {
    let mut library = ChemistryModelingLibrary::new();
    library.initialize().unwrap();

    // REAL: the full library facade runs an RHF/STO-3G calculation for H2 at
    // R = 1.4 bohr and returns a genuine SCF energy (ref ≈ −1.1167 Hartree).
    let h2 = molecule(vec![
        atom("Ha", "H", 1, [0.0, 0.0, 0.0]),
        atom("Hb", "H", 1, [1.4, 0.0, 0.0]),
    ]);
    let result = library
        .calculate_quantum_properties(h2, QuantumMethodType::HartreeFock)
        .expect("H2 quantum properties should be computed");
    assert!(
        (result.result.total_energy - (-1.1167)).abs() < 1e-3,
        "facade H2 SCF energy = {}",
        result.result.total_energy
    );

    // HONEST: an element beyond H/He (Molecule::new() is a carbon atom) still
    // reports NotImplemented rather than fabricating an energy.
    let unsupported =
        library.calculate_quantum_properties(Molecule::new(), QuantumMethodType::HartreeFock);
    assert!(matches!(
        unsupported,
        Err(ChemistryError::NotImplemented(_))
    ));
}

#[test]
fn test_reaction_kinetics() {
    let mut library = ChemistryModelingLibrary::new();
    library.initialize().unwrap();

    let reaction = Reaction::new(); // 1 step: A=1.0, Ea=10.0 kJ/mol; reactant "A"
    let conditions = ReactionConditions::new(); // T = 298.15 K

    let result = library
        .analyze_reaction_kinetics(reaction, conditions)
        .unwrap();

    // Verify the REAL Arrhenius value, not just ">0": k = A·exp(−Ea/(R·T)).
    const R: f64 = 8.314_462_618;
    let expected_k = 1.0 * (-10_000.0 / (R * 298.15)).exp();
    assert!(
        (result.result.rate_constant - expected_k).abs() < 1e-12,
        "k = {} != Arrhenius {}",
        result.result.rate_constant,
        expected_k
    );
    assert_eq!(result.result.reaction_order, 1);
    // First-order half-life t½ = ln2 / k.
    assert!((result.result.half_life - std::f64::consts::LN_2 / expected_k).abs() < 1e-9);
    // Higher temperature ⇒ larger rate constant (Arrhenius monotonicity).
    let mut hot = ReactionConditions::new();
    hot.temperature = 400.0;
    let k_hot = library
        .analyze_reaction_kinetics(Reaction::new(), hot)
        .unwrap()
        .result
        .rate_constant;
    assert!(k_hot > result.result.rate_constant);
}

#[test]
fn test_property_prediction() {
    let mut library = ChemistryModelingLibrary::new();
    library.initialize().unwrap();

    let molecule = Molecule::new(); // single C atom (mass 12.01)
    let properties = vec![PropertyType::BoilingPoint];

    // REAL: a group-contribution QSPR model for boiling point is now
    // registered, so the prediction returns a finite value rather than
    // NotImplemented. Tb = 198.2 + 23.97·(C count) = 222.17 for one carbon.
    let result = library.predict_properties(molecule, properties);
    let predicted = result.unwrap();
    let tb = *predicted.result.properties.get("boiling_point").unwrap();
    assert!(tb.is_finite());
    assert!((tb - 222.17).abs() < 1e-6, "boiling point {}", tb);
}

#[test]
fn test_performance_metrics() {
    let library = ChemistryModelingLibrary::new();
    let metrics = library.get_performance_stats();

    assert_eq!(metrics.simulation_metrics.total_simulations, 0);
    assert_eq!(metrics.quantum_metrics.total_calculations, 0);
    assert_eq!(metrics.reaction_metrics.total_reactions, 0);
    assert_eq!(metrics.property_metrics.total_predictions, 0);
}

#[test]
fn test_force_field_listing() {
    let library = ChemistryModelingLibrary::new();
    let force_fields = library.list_force_fields();

    assert!(force_fields.contains(&"AMBER".to_string()));
    assert!(force_fields.contains(&"CHARMM".to_string()));
    assert!(force_fields.contains(&"OPLS".to_string()));
}

#[test]
fn test_molecule_info() {
    let library = ChemistryModelingLibrary::new();
    let info = library.get_molecule_info("mol_1");
    assert!(info.is_none());
}

#[test]
fn test_force_field_calculator_initialization() {
    let mut calc = ForceFieldCalculator::new();
    assert!(calc.initialize().is_ok());

    // All five standard force fields should be registered.
    let names = calc.list_force_fields();
    assert!(names.contains(&"AMBER".to_string()));
    assert!(names.contains(&"CHARMM".to_string()));
    assert!(names.contains(&"OPLS".to_string()));
    assert!(names.contains(&"GROMOS".to_string()));
    assert!(names.contains(&"Universal".to_string()));
    assert_eq!(names.len(), 5);

    // Accessor returns the right typed entry.
    let amber = calc.get_force_field("AMBER").unwrap();
    assert_eq!(amber.field_type, ForceFieldType::AMBER);
    assert_eq!(amber.field_name, "AMBER");
    // Each parameter vector has at least one default entry.
    assert!(!amber.parameters.bond_parameters.is_empty());
    assert!(!amber.parameters.angle_parameters.is_empty());
    assert!(!amber.parameters.torsion_parameters.is_empty());
    assert!(!amber.parameters.nonbonded_parameters.is_empty());

    // Unknown force field lookup returns None.
    assert!(calc.get_force_field("nonexistent").is_none());
}

#[test]
fn test_custom_force_field_registration() {
    let mut calc = ForceFieldCalculator::new();
    calc.initialize().unwrap();

    let custom = ForceField {
        field_id: "ff_custom".to_string(),
        field_name: "MyFF".to_string(),
        field_type: ForceFieldType::Custom,
        parameters: ForceFieldParameters::new(),
    };
    calc.register_force_field("MyFF", custom);

    assert!(calc.get_force_field("MyFF").is_some());
    let names = calc.list_force_fields();
    assert!(names.contains(&"MyFF".to_string()));
    assert_eq!(names.len(), 6);
}

#[test]
fn test_ensemble_manager_initialization() {
    let mut manager = EnsembleManager::new();
    assert!(manager.initialize().is_ok());

    // Standard ensembles.
    let ensembles = manager.list_ensembles();
    assert!(ensembles.contains(&"NVE".to_string()));
    assert!(ensembles.contains(&"NVT".to_string()));
    assert!(ensembles.contains(&"NPT".to_string()));
    assert!(ensembles.contains(&"GCMC".to_string()));
    assert_eq!(ensembles.len(), 4);
    assert_eq!(manager.get_ensemble("NVT"), Some(&Ensemble::NVT));
    assert_eq!(manager.get_ensemble("GCMC"), Some(&Ensemble::MuVT));
    assert!(manager.get_ensemble("nonexistent").is_none());

    // Standard transition methods.
    let transitions = manager.list_transitions();
    assert!(transitions.contains(&"Berendsen".to_string()));
    assert!(transitions.contains(&"Nosé-Hoover".to_string()));
    assert!(transitions.contains(&"Parrinello-Rahman".to_string()));
    assert!(transitions.contains(&"Langevin".to_string()));
    assert_eq!(transitions.len(), 4);

    // Standard sampling methods.
    let sampling = manager.list_sampling_methods();
    assert!(sampling.contains(&"Metropolis".to_string()));
    assert!(sampling.contains(&"Gibbs".to_string()));
    assert!(sampling.contains(&"Hamiltonian".to_string()));
    assert!(sampling.contains(&"ParallelTempering".to_string()));
    assert_eq!(sampling.len(), 4);
}

#[test]
fn test_qspr_boiling_point_prediction() {
    let mut predictor = PropertyPredictor::new();
    predictor.initialize().unwrap();

    // Methane-style descriptors: 1 carbon, 4 hydrogens.
    // Tb = 198.2 + 23.97·1 + 22.88·4 = 313.69
    let mut descriptors = HashMap::new();
    descriptors.insert("C".to_string(), 1.0);
    descriptors.insert("H".to_string(), 4.0);

    let tb = predictor.predict("boiling_point", &descriptors).unwrap();
    let expected = 198.2 + 23.97 * 1.0 + 22.88 * 4.0;
    assert!(
        (tb - expected).abs() < 1e-9,
        "predicted {} != {}",
        tb,
        expected
    );
    assert!(tb.is_finite() && tb > 0.0);
}

#[test]
fn test_qspr_solubility_prediction() {
    let mut predictor = PropertyPredictor::new();
    predictor.initialize().unwrap();

    // logS = -0.5·logP - 0.01·MW + 0.5
    let mut descriptors = HashMap::new();
    descriptors.insert("logP".to_string(), 2.0);
    descriptors.insert("molecular_weight".to_string(), 100.0);

    let logs = predictor.predict("solubility", &descriptors).unwrap();
    let expected = -0.5 * 2.0 - 0.01 * 100.0 + 0.5;
    assert!(
        (logs - expected).abs() < 1e-9,
        "predicted {} != {}",
        logs,
        expected
    );
}

#[test]
fn test_qspr_molecular_weight_prediction() {
    let mut predictor = PropertyPredictor::new();
    predictor.initialize().unwrap();

    // Water: 2 H + 1 O → MW = 2·1.008 + 15.999 = 18.015
    let mut descriptors = HashMap::new();
    descriptors.insert("H".to_string(), 2.0);
    descriptors.insert("O".to_string(), 1.0);

    let mw = predictor.predict("molecular_weight", &descriptors).unwrap();
    let expected = 2.0 * 1.008 + 1.0 * 15.999;
    assert!(
        (mw - expected).abs() < 1e-9,
        "predicted {} != {}",
        mw,
        expected
    );
}

#[test]
fn test_property_predictor_unknown_property_returns_error() {
    let mut predictor = PropertyPredictor::new();
    predictor.initialize().unwrap();

    let descriptors = HashMap::new();
    let result = predictor.predict("nonexistent_property", &descriptors);
    assert!(matches!(result, Err(ChemistryError::NotImplemented(_))));

    // list_properties reports the registered models only.
    let props = predictor.list_properties();
    assert!(props.contains(&"boiling_point".to_string()));
    assert!(props.contains(&"melting_point".to_string()));
    assert!(props.contains(&"solubility".to_string()));
    assert!(props.contains(&"molecular_weight".to_string()));
    assert!(!props.contains(&"nonexistent_property".to_string()));
}

#[test]
fn test_property_predictor_register_custom_model() {
    let mut predictor = PropertyPredictor::new();
    predictor.initialize().unwrap();

    // Custom linear model: y = 2.0·x + 1.0
    let mut coeffs = HashMap::new();
    coeffs.insert("intercept".to_string(), 1.0);
    coeffs.insert("x".to_string(), 2.0);
    let model = PropertyModel {
        model_id: "custom_linear".to_string(),
        property_type: PropertyType::Density,
        model_type: PropertyModelType::GroupContribution,
        parameters: PropertyModelParameters {
            coefficients: coeffs,
            descriptors: vec!["x".to_string()],
            reference_data: Vec::new(),
        },
    };
    predictor.register_model("custom", model);

    let mut descriptors = HashMap::new();
    descriptors.insert("x".to_string(), 5.0);
    let y = predictor.predict("custom", &descriptors).unwrap();
    assert!((y - 11.0).abs() < 1e-9, "predicted {} != 11.0", y);
}

#[test]
fn test_attach_dependencies_wiring() {
    // Constructing without dependencies must still work (zero-arg new).
    let mut library = ChemistryModelingLibrary::new();
    assert!(library.initialize().is_ok());

    // Attaching Phase 2 dependencies should succeed and not break operation.
    // `ZnsZoneManager::new` opens a real device path, so point it at a
    // temporary file that exists and is read/writable.
    let zns_path = std::env::temp_dir().join("qualia_chem_zns_test_device");
    std::fs::write(&zns_path, b"zns").unwrap();
    let zns = ZnsZoneManager::new(&zns_path)
        .ok()
        .map(|m| Arc::new(Mutex::new(m)));
    let _ = std::fs::remove_file(&zns_path);

    let la = Arc::new(Mutex::new(LinearAlgebraLibrary::new()));
    let sc = Arc::new(Mutex::new(StatisticalComputingLibrary::new()));
    let csd = Arc::new(Mutex::new(CsdManager::new()));

    if let Some(zns) = zns {
        library.attach_dependencies(la, sc, csd, zns);
        // Re-initializing after attaching should still succeed.
        assert!(library.initialize().is_ok());
    }
    // When no ZNS device is available the library must still work without
    // dependencies attached (covered by the other tests above).
}

// ─── Exact structural / mass property tests (known values) ─────────────

/// Build an atom with a real element, Z, and coordinates. Mass is taken from
/// the standard-weight table so the helper stays honest.
fn atom(id: &str, element: &str, z: usize, coords: [f64; 3]) -> Atom {
    Atom {
        atom_id: id.to_string(),
        element: element.to_string(),
        atomic_number: z,
        mass: standard_atomic_weight(element).unwrap_or(0.0),
        charge: 0.0,
        coordinates: coords.to_vec(),
    }
}

fn molecule(atoms: Vec<Atom>) -> Molecule {
    Molecule {
        molecule_id: "test".to_string(),
        formula: String::new(),
        atoms,
        bonds: Vec::new(),
        coordinates: Vec::new(),
        properties: MolecularProperties::new(),
    }
}

#[test]
fn test_molecular_mass_water() {
    let lib = ChemistryModelingLibrary::new();
    let h2o = molecule(vec![
        atom("O", "O", 8, [0.0, 0.0, 0.0]),
        atom("H1", "H", 1, [0.757, 0.586, 0.0]),
        atom("H2", "H", 1, [-0.757, 0.586, 0.0]),
    ]);
    // 15.999 + 2·1.008 = 18.015 amu (standard atomic weights).
    let m = lib.molecular_mass(&h2o);
    assert!((m - 18.015).abs() < 1e-9, "H2O mass = {}", m);
}

#[test]
fn test_molecular_formula_hill() {
    let lib = ChemistryModelingLibrary::new();
    // Water: no carbon → alphabetical H, O.
    let h2o = molecule(vec![
        atom("O", "O", 8, [0.0, 0.0, 0.0]),
        atom("H1", "H", 1, [1.0, 0.0, 0.0]),
        atom("H2", "H", 1, [0.0, 1.0, 0.0]),
    ]);
    assert_eq!(lib.molecular_formula(&h2o), "H2O");
    // Ethanol C2H6O: carbon first, then hydrogen, then O.
    let mut atoms = vec![
        atom("C1", "C", 6, [0.0, 0.0, 0.0]),
        atom("C2", "C", 6, [1.5, 0.0, 0.0]),
        atom("O", "O", 8, [2.0, 1.0, 0.0]),
    ];
    for i in 0..6 {
        atoms.push(atom(&format!("H{i}"), "H", 1, [i as f64, 2.0, 0.0]));
    }
    assert_eq!(lib.molecular_formula(&molecule(atoms)), "C2H6O");
}

#[test]
fn test_nuclear_repulsion_diatomic() {
    let lib = ChemistryModelingLibrary::new();
    // HeH: Z=2, Z=1 at r = 2.0 bohr → E_nn = 2·1/2 = 1.0 Hartree.
    let heh = molecule(vec![
        atom("He", "He", 2, [0.0, 0.0, 0.0]),
        atom("H", "H", 1, [2.0, 0.0, 0.0]),
    ]);
    let e = lib.nuclear_repulsion_energy(&heh).unwrap();
    assert!((e - 1.0).abs() < 1e-12, "E_nn(HeH, r=2) = {}", e);

    // H2 at r = 1.4 bohr → 1·1/1.4.
    let h2 = molecule(vec![
        atom("Ha", "H", 1, [0.0, 0.0, 0.0]),
        atom("Hb", "H", 1, [1.4, 0.0, 0.0]),
    ]);
    let e2 = lib.nuclear_repulsion_energy(&h2).unwrap();
    assert!((e2 - 1.0 / 1.4).abs() < 1e-12, "E_nn(H2, r=1.4) = {}", e2);

    // Single atom: no pairs → 0.
    let he = molecule(vec![atom("He", "He", 2, [0.0, 0.0, 0.0])]);
    assert_eq!(lib.nuclear_repulsion_energy(&he).unwrap(), 0.0);

    // Zero nuclear charge must be refused, not faked.
    let bad = molecule(vec![
        atom("X", "X", 0, [0.0, 0.0, 0.0]),
        atom("H", "H", 1, [1.0, 0.0, 0.0]),
    ]);
    assert!(matches!(
        lib.nuclear_repulsion_energy(&bad),
        Err(ChemistryError::InsufficientData(_))
    ));
}

#[test]
fn test_bond_length_and_angle() {
    let lib = ChemistryModelingLibrary::new();
    // Right angle at the origin: A=(1,0,0), vertex=(0,0,0), C=(0,1,0).
    let m = molecule(vec![
        atom("A", "H", 1, [1.0, 0.0, 0.0]),
        atom("V", "O", 8, [0.0, 0.0, 0.0]),
        atom("C", "H", 1, [0.0, 1.0, 0.0]),
    ]);
    let d = lib.bond_length(&m, 0, 1).unwrap();
    assert!((d - 1.0).abs() < 1e-12, "bond length = {}", d);
    let theta = lib.bond_angle(&m, 0, 1, 2).unwrap();
    assert!(
        (theta - std::f64::consts::FRAC_PI_2).abs() < 1e-12,
        "angle = {} rad",
        theta
    );

    // 3-4-5 triangle leg gives a length of 5.
    let m2 = molecule(vec![
        atom("A", "H", 1, [0.0, 0.0, 0.0]),
        atom("B", "H", 1, [3.0, 4.0, 0.0]),
    ]);
    assert!((lib.bond_length(&m2, 0, 1).unwrap() - 5.0).abs() < 1e-12);
}

#[test]
fn test_center_of_mass_and_inertia() {
    let lib = ChemistryModelingLibrary::new();
    // Two equal H masses on the x-axis at ±d/2, d = 2.0 → COM at origin.
    let d = 2.0;
    let m = molecule(vec![
        atom("Ha", "H", 1, [-d / 2.0, 0.0, 0.0]),
        atom("Hb", "H", 1, [d / 2.0, 0.0, 0.0]),
    ]);
    let com = lib.center_of_mass(&m).unwrap();
    assert!(com.iter().all(|c| c.abs() < 1e-12), "COM = {:?}", com);

    // Inertia about the molecular (x) axis is 0; the two perpendicular axes
    // are each 2·m·(d/2)² with m = 1.008.
    let mass = standard_atomic_weight("H").unwrap();
    let i_perp = 2.0 * mass * (d / 2.0) * (d / 2.0);
    let moments = lib.principal_moments_of_inertia(&m).unwrap();
    assert!(moments[0].abs() < 1e-9, "I_axial = {}", moments[0]);
    assert!(
        (moments[1] - i_perp).abs() < 1e-9 && (moments[2] - i_perp).abs() < 1e-9,
        "I_perp = ({}, {}), expected {}",
        moments[1],
        moments[2],
        i_perp
    );
}

#[test]
fn test_structural_properties_aggregate() {
    let lib = ChemistryModelingLibrary::new();
    let h2 = molecule(vec![
        atom("Ha", "H", 1, [0.0, 0.0, 0.0]),
        atom("Hb", "H", 1, [1.4, 0.0, 0.0]),
    ]);
    let props = lib.structural_properties(&h2).unwrap();
    assert_eq!(props.formula, "H2");
    assert_eq!(props.atom_count, 2);
    assert!((props.molecular_mass - 2.0 * 1.008).abs() < 1e-9);
    assert!((props.nuclear_repulsion_energy.unwrap() - 1.0 / 1.4).abs() < 1e-12);
}

#[test]
fn test_rhf_h2_sto3g_energy() {
    // REAL RHF/STO-3G on H2 at the equilibrium bond length R = 1.4 bohr.
    // The energy falls out of a genuine SCF over analytical integrals; the
    // textbook reference is E ≈ −1.1167 Hartree (Szabo & Ostlund).
    let mut calc = QuantumCalculator::new();
    let h2 = molecule(vec![
        atom("Ha", "H", 1, [0.0, 0.0, 0.0]),
        atom("Hb", "H", 1, [1.4, 0.0, 0.0]),
    ]);
    let p = calc
        .calculate_properties(&h2, QuantumMethodType::HartreeFock)
        .expect("H2 RHF should converge");
    assert!(
        (p.total_energy - (-1.1167)).abs() < 1e-3,
        "H2 SCF energy = {} (ref -1.1167)",
        p.total_energy
    );
    // A real bonding→antibonding HOMO/LUMO gap, with a bound HOMO.
    assert!(p.gap > 0.0, "H2 HOMO-LUMO gap not positive: {}", p.gap);
    assert!(
        p.homo_energy < 0.0,
        "H2 HOMO should be bound: {}",
        p.homo_energy
    );
    assert!(
        p.lumo_energy > p.homo_energy,
        "LUMO {} must sit above HOMO {}",
        p.lumo_energy,
        p.homo_energy
    );
    // Neutral, symmetric: Mulliken charges vanish and sum to ~0.
    let sum: f64 = p.mulliken_charges.iter().sum();
    assert!(
        sum.abs() < 1e-6,
        "Mulliken charges sum = {} (should be ~0)",
        sum
    );
    for q in &p.mulliken_charges {
        assert!(
            q.abs() < 1e-6,
            "H2 atom charge should be ~0 by symmetry: {}",
            q
        );
    }
    // Homonuclear diatomic ⇒ zero dipole moment.
    assert!(
        p.dipole_moment < 1e-6,
        "H2 dipole should vanish by symmetry: {}",
        p.dipole_moment
    );
}

#[test]
fn test_rhf_he_sto3g_energy() {
    // REAL RHF/STO-3G on the He atom; textbook reference E ≈ −2.8077 Hartree.
    let mut calc = QuantumCalculator::new();
    let he = molecule(vec![atom("He", "He", 2, [0.0, 0.0, 0.0])]);
    let p = calc
        .calculate_properties(&he, QuantumMethodType::HartreeFock)
        .expect("He RHF should converge");
    assert!(
        (p.total_energy - (-2.8077)).abs() < 1e-3,
        "He SCF energy = {} (ref -2.8077)",
        p.total_energy
    );
    assert!(
        p.homo_energy < 0.0,
        "He 1s should be bound: {}",
        p.homo_energy
    );
    // Neutral atom: Mulliken population = Z, charge ~0.
    let sum: f64 = p.mulliken_charges.iter().sum();
    assert!(sum.abs() < 1e-6, "He Mulliken charge sum = {}", sum);
    // Isolated atom ⇒ no dipole.
    assert!(p.dipole_moment < 1e-9, "He dipole = {}", p.dipole_moment);
}

#[test]
fn test_rhf_unsupported_returns_not_implemented() {
    // Elements beyond H/He need p/d shells — honest NotImplemented, never a
    // fabricated energy. This is the regression guard replacing the old stub.
    let mut calc = QuantumCalculator::new();
    let c = molecule(vec![atom("C", "C", 6, [0.0, 0.0, 0.0])]);
    assert!(matches!(
        calc.calculate_properties(&c, QuantumMethodType::HartreeFock),
        Err(ChemistryError::NotImplemented(_))
    ));
    // Odd electron count (a single H) is open-shell ⇒ NotImplemented (RHF only).
    let h = molecule(vec![atom("H", "H", 1, [0.0, 0.0, 0.0])]);
    assert!(matches!(
        calc.calculate_properties(&h, QuantumMethodType::HartreeFock),
        Err(ChemistryError::NotImplemented(_))
    ));
}
