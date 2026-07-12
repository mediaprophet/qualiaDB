use super::*;

/// Molecular simulator for molecular dynamics simulations
pub struct MolecularSimulator {
    simulation_engine: SimulationEngine,
    force_field_calculator: ForceFieldCalculator,
    integrator: MolecularIntegrator,
    boundary_conditions: BoundaryConditions,
    molecule_store: HashMap<String, Molecule>,
    linear_algebra: Option<Arc<Mutex<LinearAlgebraLibrary>>>,
    statistical_computing: Option<Arc<Mutex<StatisticalComputingLibrary>>>,
}

/// Simulation engine
pub struct SimulationEngine {
    simulation_config: SimulationConfig,
    time_step_control: TimeStepControl,
    ensemble_manager: EnsembleManager,
    temperature_controller: TemperatureController,
}

/// Simulation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub simulation_id: String,
    pub simulation_type: SimulationType,
    pub ensemble: Ensemble,
    pub time_step: f64,
    pub total_time: f64,
    pub temperature: f64,
    pub pressure: f64,
    pub box_size: Vec<f64>,
    pub boundary_type: BoundaryType,
}

/// Simulation types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SimulationType {
    /// Molecular dynamics
    MolecularDynamics,
    /// Monte Carlo
    MonteCarlo,
    /// Hybrid MD/MC
    Hybrid,
    /// Enhanced sampling
    EnhancedSampling,
    /// Coarse-grained
    CoarseGrained,
    /// Quantum mechanics/molecular mechanics
    QMMM,
}

/// Ensembles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Ensemble {
    NVE,  // Microcanonical
    NVT,  // Canonical
    NPT,  // Isothermal-isobaric
    NPH,  // Isoenthalpic-isobaric
    MuVT, // Grand canonical
}

/// Boundary types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BoundaryType {
    Periodic,
    NonPeriodic,
    SemiPeriodic,
    Ewald,
    Boiling,
}

/// Time step control
pub struct TimeStepControl {
    control_type: TimeStepControlType,
    adaptive_parameters: AdaptiveParameters,
    stability_analysis: StabilityAnalysis,
}

/// Time step control types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimeStepControlType {
    Fixed,
    Adaptive,
    Variable,
    Multiple,
}

/// Adaptive parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveParameters {
    pub min_time_step: f64,
    pub max_time_step: f64,
    pub safety_factor: f64,
    pub max_force: f64,
}

/// Stability analysis
pub struct StabilityAnalysis {
    analysis_method: StabilityAnalysisMethod,
    energy_conservation: EnergyConservation,
    temperature_fluctuation: TemperatureFluctuation,
}

/// Stability analysis methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StabilityAnalysisMethod {
    EnergyDrift,
    TemperatureDrift,
    PressureDrift,
    ConservationLaws,
}

/// Energy conservation
#[derive(Debug, Clone)]
pub struct EnergyConservation {
    pub total_energy: f64,
    pub kinetic_energy: f64,
    pub potential_energy: f64,
    pub drift_rate: f64,
}

/// Temperature fluctuation
#[derive(Debug, Clone)]
pub struct TemperatureFluctuation {
    pub current_temperature: f64,
    pub target_temperature: f64,
    pub fluctuation_amplitude: f64,
    pub heat_capacity: f64,
}

/// Ensemble manager
pub struct EnsembleManager {
    ensembles: HashMap<String, Ensemble>,
    ensemble_transitions: HashMap<String, EnsembleTransition>,
    sampling_methods: HashMap<String, SamplingMethod>,
}

/// Ensemble transitions
#[derive(Debug, Clone)]
pub struct EnsembleTransition {
    pub transition_id: String,
    pub from_ensemble: Ensemble,
    pub to_ensemble: Ensemble,
    pub transition_method: TransitionMethod,
}

/// Transition methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransitionMethod {
    Berendsen,
    NoséHoover,
    Andersen,
    ParrinelloRahman,
    MartynaTuckerman,
    /// Langevin dynamics thermostat (stochastic damping).
    Langevin,
}

/// Sampling methods
#[derive(Debug, Clone)]
pub struct SamplingMethod {
    pub method_id: String,
    pub method_type: SamplingMethodType,
    pub parameters: SamplingParameters,
}

/// Sampling method types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SamplingMethodType {
    Metropolis,
    Gibbs,
    WangLandau,
    Umbrella,
    ReplicaExchange,
    /// Hamiltonian (Hybrid) Monte Carlo — uses molecular dynamics proposals.
    Hamiltonian,
    /// Parallel tempering (replica exchange over a temperature ladder).
    ParallelTempering,
}

/// Sampling parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingParameters {
    pub acceptance_ratio: f64,
    pub proposal_width: f64,
    pub equilibration_steps: u32,
    pub production_steps: u32,
}

/// Temperature controller
pub struct TemperatureController {
    control_method: TemperatureControlMethod,
    thermostat_parameters: ThermostatParameters,
    temperature_profile: TemperatureProfile,
}

/// Temperature control methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TemperatureControlMethod {
    VelocityRescaling,
    Berendsen,
    NoséHoover,
    Langevin,
    Andersen,
}

/// Thermostat parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermostatParameters {
    pub coupling_constant: f64,
    pub relaxation_time: f64,
    pub damping_coefficient: f64,
}

/// Temperature profile
#[derive(Debug, Clone)]
pub struct TemperatureProfile {
    pub profile_type: TemperatureProfileType,
    pub initial_temperature: f64,
    pub final_temperature: Option<f64>,
    pub ramp_rate: Option<f64>,
}

/// Temperature profile types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TemperatureProfileType {
    Constant,
    Linear,
    Exponential,
    Step,
    Custom,
}

/// Force field calculator
pub struct ForceFieldCalculator {
    force_fields: HashMap<String, ForceField>,
    interaction_calculator: InteractionCalculator,
    energy_calculator: EnergyCalculator,
}

/// Force fields
#[derive(Debug, Clone)]
pub struct ForceField {
    pub field_id: String,
    pub field_name: String,
    pub field_type: ForceFieldType,
    pub parameters: ForceFieldParameters,
}

/// Force field types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ForceFieldType {
    AMBER,
    CHARMM,
    OPLS,
    GROMOS,
    DREIDING,
    MMFF,
    ReaxFF,
    Custom,
}

/// Force field parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForceFieldParameters {
    pub bond_parameters: Vec<BondParameter>,
    pub angle_parameters: Vec<AngleParameter>,
    pub torsion_parameters: Vec<TorsionParameter>,
    pub nonbonded_parameters: Vec<NonbondedParameter>,
}

/// Bond parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondParameter {
    pub atom_types: Vec<String>,
    pub equilibrium_length: f64,
    pub force_constant: f64,
}

/// Angle parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AngleParameter {
    pub atom_types: Vec<String>,
    pub equilibrium_angle: f64,
    pub force_constant: f64,
}

/// Torsion parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorsionParameter {
    pub atom_types: Vec<String>,
    pub barriers: Vec<f64>,
    pub phases: Vec<f64>,
    pub periodicities: Vec<i32>,
}

/// Nonbonded parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonbondedParameter {
    pub atom_type: String,
    pub sigma: f64,
    pub epsilon: f64,
    pub charge: f64,
}

/// Interaction calculator
pub struct InteractionCalculator {
    bonded_interactions: BondedInteractions,
    nonbonded_interactions: NonbondedInteractions,
    long_range_interactions: LongRangeInteractions,
}

/// Bonded interactions
pub struct BondedInteractions {
    bond_calculator: BondCalculator,
    angle_calculator: AngleCalculator,
    torsion_calculator: TorsionCalculator,
    improper_calculator: ImproperCalculator,
}

/// Bond calculator
#[derive(Debug, Clone)]
pub struct BondCalculator {
    pub calculator_type: BondCalculatorType,
    pub parameters: BondCalculatorParameters,
}

/// Bond calculator types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BondCalculatorType {
    Harmonic,
    Morse,
    FENE,
    Custom,
}

/// Bond calculator parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondCalculatorParameters {
    pub force_constant: f64,
    pub equilibrium_length: f64,
    pub dissociation_energy: Option<f64>,
}

/// Angle calculator
#[derive(Debug, Clone)]
pub struct AngleCalculator {
    pub calculator_type: AngleCalculatorType,
    pub parameters: AngleCalculatorParameters,
}

/// Angle calculator types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AngleCalculatorType {
    Harmonic,
    Cosine,
    UreyBradley,
    Custom,
}

/// Angle calculator parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AngleCalculatorParameters {
    pub force_constant: f64,
    pub equilibrium_angle: f64,
    pub ub_parameters: Option<UBParameters>,
}

/// UB parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UBParameters {
    pub force_constant: f64,
    pub equilibrium_length: f64,
}

/// Torsion calculator
#[derive(Debug, Clone)]
pub struct TorsionCalculator {
    pub calculator_type: TorsionCalculatorType,
    pub parameters: TorsionCalculatorParameters,
}

/// Torsion calculator types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TorsionCalculatorType {
    Cosine,
    Fourier,
    RyckaertsBellemans,
    Custom,
}

/// Torsion calculator parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorsionCalculatorParameters {
    pub barriers: Vec<f64>,
    pub phases: Vec<f64>,
    pub periodicities: Vec<i32>,
}

/// Improper calculator
#[derive(Debug, Clone)]
pub struct ImproperCalculator {
    pub calculator_type: ImproperCalculatorType,
    pub parameters: ImproperCalculatorParameters,
}

/// Improper calculator types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImproperCalculatorType {
    Harmonic,
    Cosine,
    Custom,
}

/// Improper calculator parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImproperCalculatorParameters {
    pub force_constant: f64,
    pub equilibrium_angle: f64,
}

/// Nonbonded interactions
pub struct NonbondedInteractions {
    lennard_jones: LennardJones,
    coulomb: Coulomb,
    buckingham: Buckingham,
}

/// Lennard-Jones potential
#[derive(Debug, Clone)]
pub struct LennardJones {
    pub epsilon: f64,
    pub sigma: f64,
    pub cutoff: f64,
    pub switching_distance: f64,
}

/// Coulomb potential
#[derive(Debug, Clone)]
pub struct Coulomb {
    pub coulomb_constant: f64,
    pub dielectric: f64,
    pub cutoff: f64,
    pub switching_distance: f64,
}

/// Buckingham potential
#[derive(Debug, Clone)]
pub struct Buckingham {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub cutoff: f64,
}

/// Long-range interactions
pub struct LongRangeInteractions {
    ewald_summation: EwaldSummation,
    particle_mesh: ParticleMesh,
    reaction_field: ReactionField,
}

/// Ewald summation
#[derive(Debug, Clone)]
pub struct EwaldSummation {
    pub alpha: f64,
    pub k_max: usize,
    pub real_cutoff: f64,
    pub reciprocal_cutoff: f64,
}

/// Particle mesh
#[derive(Debug, Clone)]
pub struct ParticleMesh {
    pub grid_size: Vec<usize>,
    pub spline_order: usize,
    pub cutoff: f64,
}

/// Reaction field
#[derive(Debug, Clone)]
pub struct ReactionField {
    pub dielectric_inside: f64,
    pub dielectric_outside: f64,
    pub cutoff: f64,
}

/// Energy calculator
pub struct EnergyCalculator {
    kinetic_energy: KineticEnergy,
    potential_energy: PotentialEnergy,
    total_energy: TotalEnergy,
}

/// Kinetic energy
#[derive(Debug, Clone)]
pub struct KineticEnergy {
    pub temperature: f64,
    pub degrees_of_freedom: usize,
    pub velocities: Vec<Vec<f64>>,
}

/// Potential energy
#[derive(Debug, Clone)]
pub struct PotentialEnergy {
    pub bonded_energy: f64,
    pub nonbonded_energy: f64,
    pub long_range_energy: f64,
}

/// Total energy
#[derive(Debug, Clone)]
pub struct TotalEnergy {
    pub kinetic: f64,
    pub potential: f64,
    pub total: f64,
    pub drift: f64,
}

/// Molecular integrator
pub struct MolecularIntegrator {
    integrator_type: IntegratorType,
    integrator_parameters: IntegratorParameters,
    constraint_handler: ConstraintHandler,
}

/// Integrator types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IntegratorType {
    VelocityVerlet,
    Leapfrog,
    Beeman,
    Gear,
    RungeKutta,
    Stochastic,
}

/// Integrator parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegratorParameters {
    pub time_step: f64,
    pub accuracy: f64,
    pub stability_factor: f64,
}

/// Constraint handler
pub struct ConstraintHandler {
    constraint_algorithm: ConstraintAlgorithm,
    constraint_parameters: ConstraintParameters,
}

/// Constraint algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintAlgorithm {
    SHAKE,
    RATTLE,
    LINCS,
    SETTLE,
}

/// Constraint parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintParameters {
    pub tolerance: f64,
    pub max_iterations: u32,
    pub relaxation_parameter: f64,
}

/// Boundary conditions
pub struct BoundaryConditions {
    boundary_type: BoundaryType,
    box_vectors: Vec<Vec<f64>>,
    minimum_image: MinimumImage,
}

/// Minimum image convention
#[derive(Debug, Clone)]
pub struct MinimumImage {
    pub box_size: Vec<f64>,
    pub periodic: bool,
}

// Supporting implementations

impl MolecularSimulator {
    pub fn new() -> Self {
        Self {
            simulation_engine: SimulationEngine::new(),
            force_field_calculator: ForceFieldCalculator::new(),
            integrator: MolecularIntegrator::new(),
            boundary_conditions: BoundaryConditions::new(),
            molecule_store: HashMap::new(),
            linear_algebra: None,
            statistical_computing: None,
        }
    }

    pub fn attach_dependencies(
        &mut self,
        linear_algebra: Option<Arc<Mutex<LinearAlgebraLibrary>>>,
        statistical_computing: Option<Arc<Mutex<StatisticalComputingLibrary>>>,
    ) {
        self.linear_algebra = linear_algebra;
        self.statistical_computing = statistical_computing;
    }

    pub fn store_molecule(&mut self, molecule: Molecule) {
        self.molecule_store
            .insert(molecule.molecule_id.clone(), molecule);
    }

    pub fn initialize(&mut self) -> Result<(), ChemistryError> {
        self.simulation_engine.initialize()?;
        self.force_field_calculator.initialize()?;
        self.integrator.initialize()?;

        let _ = self.boundary_conditions.boundary_type();
        let _ = self.boundary_conditions.box_vectors();
        let _ = self.boundary_conditions.minimum_image();

        Ok(())
    }

    pub fn validate_config(&self, config: &SimulationConfig) -> Result<(), ChemistryError> {
        if config.time_step <= 0.0 {
            return Err(ChemistryError::ValidationError(
                "Time step must be positive".to_string(),
            ));
        }
        if config.total_time <= 0.0 {
            return Err(ChemistryError::ValidationError(
                "Total time must be positive".to_string(),
            ));
        }
        if config.temperature < 0.0 {
            return Err(ChemistryError::ValidationError(
                "Temperature must be non-negative".to_string(),
            ));
        }
        Ok(())
    }

    pub fn run_simulation(
        &mut self,
        config: &SimulationConfig,
        molecule: &Molecule,
    ) -> Result<SimulationTrajectory, ChemistryError> {
        // REAL: Lennard-Jones force field + velocity-Verlet integrator, in
        // `molecular_dynamics::run_md`. The atoms actually move under computed
        // forces, total energy is conserved (asserted in that module's tests),
        // and invalid inputs (no atoms / unparameterized element / bad mass)
        // return `InsufficientData` rather than a fabricated static trajectory.
        molecular_dynamics::run_md(
            config,
            molecule,
            self.linear_algebra.clone(),
            self.statistical_computing.clone(),
        )
    }

    pub fn list_force_fields(&self) -> Vec<String> {
        vec![
            "AMBER".to_string(),
            "CHARMM".to_string(),
            "OPLS".to_string(),
        ]
    }

    pub fn get_molecule(&self, molecule_id: &str) -> Option<Molecule> {
        self.molecule_store.get(molecule_id).cloned()
    }

    /// Borrow the boundary-conditions configuration.
    pub fn boundary_conditions(&self) -> &BoundaryConditions {
        &self.boundary_conditions
    }

    /// Mutably borrow the boundary-conditions configuration.
    pub fn boundary_conditions_mut(&mut self) -> &mut BoundaryConditions {
        &mut self.boundary_conditions
    }

    /// Borrow the simulation engine.
    pub fn simulation_engine(&self) -> &SimulationEngine {
        &self.simulation_engine
    }

    /// Mutably borrow the simulation engine.
    pub fn simulation_engine_mut(&mut self) -> &mut SimulationEngine {
        &mut self.simulation_engine
    }

    /// Borrow the force-field calculator.
    pub fn force_field_calculator(&self) -> &ForceFieldCalculator {
        &self.force_field_calculator
    }

    /// Mutably borrow the force-field calculator.
    pub fn force_field_calculator_mut(&mut self) -> &mut ForceFieldCalculator {
        &mut self.force_field_calculator
    }

    /// Borrow the molecular integrator.
    pub fn integrator(&self) -> &MolecularIntegrator {
        &self.integrator
    }

    /// Mutably borrow the molecular integrator.
    pub fn integrator_mut(&mut self) -> &mut MolecularIntegrator {
        &mut self.integrator
    }
}

impl SimulationEngine {
    pub fn new() -> Self {
        Self {
            simulation_config: SimulationConfig::new(),
            time_step_control: TimeStepControl::new(),
            ensemble_manager: EnsembleManager::new(),
            temperature_controller: TemperatureController::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), ChemistryError> {
        self.time_step_control.initialize()?;
        self.ensemble_manager.initialize()?;
        self.temperature_controller.initialize()?;
        Ok(())
    }

    /// Borrow the simulation configuration.
    pub fn simulation_config(&self) -> &SimulationConfig {
        &self.simulation_config
    }

    /// Mutably borrow the simulation configuration.
    pub fn simulation_config_mut(&mut self) -> &mut SimulationConfig {
        &mut self.simulation_config
    }

    /// Borrow the time-step controller.
    pub fn time_step_control(&self) -> &TimeStepControl {
        &self.time_step_control
    }

    /// Mutably borrow the time-step controller.
    pub fn time_step_control_mut(&mut self) -> &mut TimeStepControl {
        &mut self.time_step_control
    }

    /// Borrow the ensemble manager.
    pub fn ensemble_manager(&self) -> &EnsembleManager {
        &self.ensemble_manager
    }

    /// Mutably borrow the ensemble manager.
    pub fn ensemble_manager_mut(&mut self) -> &mut EnsembleManager {
        &mut self.ensemble_manager
    }

    /// Borrow the temperature controller.
    pub fn temperature_controller(&self) -> &TemperatureController {
        &self.temperature_controller
    }

    /// Mutably borrow the temperature controller.
    pub fn temperature_controller_mut(&mut self) -> &mut TemperatureController {
        &mut self.temperature_controller
    }
}

impl SimulationConfig {
    pub fn new() -> Self {
        Self {
            simulation_id: "sim_1".to_string(),
            simulation_type: SimulationType::MolecularDynamics,
            ensemble: Ensemble::NVT,
            time_step: 0.001,
            total_time: 1.0,
            temperature: 300.0,
            pressure: 1.0,
            box_size: vec![10.0, 10.0, 10.0],
            boundary_type: BoundaryType::Periodic,
        }
    }
}

impl TimeStepControl {
    pub fn new() -> Self {
        Self {
            control_type: TimeStepControlType::Fixed,
            adaptive_parameters: AdaptiveParameters::new(),
            stability_analysis: StabilityAnalysis::new(),
        }
    }

    /// Borrow the time-step control strategy.
    pub fn control_type(&self) -> &TimeStepControlType {
        &self.control_type
    }

    /// Set the time-step control strategy.
    pub fn set_control_type(&mut self, control_type: TimeStepControlType) {
        self.control_type = control_type;
    }

    /// Borrow the adaptive time-step parameters.
    pub fn adaptive_parameters(&self) -> &AdaptiveParameters {
        &self.adaptive_parameters
    }

    /// Mutably borrow the adaptive time-step parameters.
    pub fn adaptive_parameters_mut(&mut self) -> &mut AdaptiveParameters {
        &mut self.adaptive_parameters
    }

    /// Borrow the stability analysis.
    pub fn stability_analysis(&self) -> &StabilityAnalysis {
        &self.stability_analysis
    }

    /// Mutably borrow the stability analysis.
    pub fn stability_analysis_mut(&mut self) -> &mut StabilityAnalysis {
        &mut self.stability_analysis
    }

    pub fn initialize(&mut self) -> Result<(), ChemistryError> {
        Ok(())
    }
}

impl AdaptiveParameters {
    pub fn new() -> Self {
        Self {
            min_time_step: 0.0001,
            max_time_step: 0.01,
            safety_factor: 0.9,
            max_force: 1000.0,
        }
    }
}

impl StabilityAnalysis {
    pub fn new() -> Self {
        Self {
            analysis_method: StabilityAnalysisMethod::EnergyDrift,
            energy_conservation: EnergyConservation::new(),
            temperature_fluctuation: TemperatureFluctuation::new(),
        }
    }

    /// Borrow the stability analysis method.
    pub fn analysis_method(&self) -> &StabilityAnalysisMethod {
        &self.analysis_method
    }

    /// Set the stability analysis method.
    pub fn set_analysis_method(&mut self, method: StabilityAnalysisMethod) {
        self.analysis_method = method;
    }

    /// Borrow the energy-conservation metrics.
    pub fn energy_conservation(&self) -> &EnergyConservation {
        &self.energy_conservation
    }

    /// Mutably borrow the energy-conservation metrics.
    pub fn energy_conservation_mut(&mut self) -> &mut EnergyConservation {
        &mut self.energy_conservation
    }

    /// Borrow the temperature-fluctuation metrics.
    pub fn temperature_fluctuation(&self) -> &TemperatureFluctuation {
        &self.temperature_fluctuation
    }

    /// Mutably borrow the temperature-fluctuation metrics.
    pub fn temperature_fluctuation_mut(&mut self) -> &mut TemperatureFluctuation {
        &mut self.temperature_fluctuation
    }
}

impl EnergyConservation {
    pub fn new() -> Self {
        Self {
            total_energy: 0.0,
            kinetic_energy: 0.0,
            potential_energy: 0.0,
            drift_rate: 0.0,
        }
    }
}

impl TemperatureFluctuation {
    pub fn new() -> Self {
        Self {
            current_temperature: 300.0,
            target_temperature: 300.0,
            fluctuation_amplitude: 5.0,
            heat_capacity: 100.0,
        }
    }
}

impl EnsembleManager {
    pub fn new() -> Self {
        Self {
            ensembles: HashMap::new(),
            ensemble_transitions: HashMap::new(),
            sampling_methods: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), ChemistryError> {
        // Standard thermodynamic ensembles. GCMC (grand canonical) maps to the
        // `MuVT` variant.
        self.ensembles.insert("NVE".to_string(), Ensemble::NVE);
        self.ensembles.insert("NVT".to_string(), Ensemble::NVT);
        self.ensembles.insert("NPT".to_string(), Ensemble::NPT);
        self.ensembles.insert("GCMC".to_string(), Ensemble::MuVT);

        // Standard ensemble transition / thermostat methods.
        self.ensemble_transitions.insert(
            "Berendsen".to_string(),
            EnsembleTransition {
                transition_id: "trans_berendsen".to_string(),
                from_ensemble: Ensemble::NVE,
                to_ensemble: Ensemble::NVT,
                transition_method: TransitionMethod::Berendsen,
            },
        );
        self.ensemble_transitions.insert(
            "Nosé-Hoover".to_string(),
            EnsembleTransition {
                transition_id: "trans_nose_hoover".to_string(),
                from_ensemble: Ensemble::NVT,
                to_ensemble: Ensemble::NVT,
                transition_method: TransitionMethod::NoséHoover,
            },
        );
        self.ensemble_transitions.insert(
            "Parrinello-Rahman".to_string(),
            EnsembleTransition {
                transition_id: "trans_parrinello_rahman".to_string(),
                from_ensemble: Ensemble::NPT,
                to_ensemble: Ensemble::NPT,
                transition_method: TransitionMethod::ParrinelloRahman,
            },
        );
        self.ensemble_transitions.insert(
            "Langevin".to_string(),
            EnsembleTransition {
                transition_id: "trans_langevin".to_string(),
                from_ensemble: Ensemble::NVT,
                to_ensemble: Ensemble::NVT,
                transition_method: TransitionMethod::Langevin,
            },
        );

        // Standard sampling methods.
        self.sampling_methods.insert(
            "Metropolis".to_string(),
            SamplingMethod {
                method_id: "sample_metropolis".to_string(),
                method_type: SamplingMethodType::Metropolis,
                parameters: SamplingParameters::new(),
            },
        );
        self.sampling_methods.insert(
            "Gibbs".to_string(),
            SamplingMethod {
                method_id: "sample_gibbs".to_string(),
                method_type: SamplingMethodType::Gibbs,
                parameters: SamplingParameters::new(),
            },
        );
        self.sampling_methods.insert(
            "Hamiltonian".to_string(),
            SamplingMethod {
                method_id: "sample_hmc".to_string(),
                method_type: SamplingMethodType::Hamiltonian,
                parameters: SamplingParameters::new(),
            },
        );
        self.sampling_methods.insert(
            "ParallelTempering".to_string(),
            SamplingMethod {
                method_id: "sample_pt".to_string(),
                method_type: SamplingMethodType::ParallelTempering,
                parameters: SamplingParameters::new(),
            },
        );

        Ok(())
    }

    /// Look up a registered ensemble by name.
    pub fn get_ensemble(&self, name: &str) -> Option<&Ensemble> {
        self.ensembles.get(name)
    }

    /// List the names of all registered ensembles.
    pub fn list_ensembles(&self) -> Vec<String> {
        self.ensembles.keys().cloned().collect()
    }

    /// List the names of all registered ensemble transition methods.
    pub fn list_transitions(&self) -> Vec<String> {
        self.ensemble_transitions.keys().cloned().collect()
    }

    /// List the names of all registered sampling methods.
    pub fn list_sampling_methods(&self) -> Vec<String> {
        self.sampling_methods.keys().cloned().collect()
    }
}

impl EnsembleTransition {
    pub fn new() -> Self {
        Self {
            transition_id: "transition_1".to_string(),
            from_ensemble: Ensemble::NVE,
            to_ensemble: Ensemble::NVT,
            transition_method: TransitionMethod::Berendsen,
        }
    }
}

impl SamplingMethod {
    pub fn new() -> Self {
        Self {
            method_id: "method_1".to_string(),
            method_type: SamplingMethodType::Metropolis,
            parameters: SamplingParameters::new(),
        }
    }
}

impl SamplingParameters {
    pub fn new() -> Self {
        Self {
            acceptance_ratio: 0.5,
            proposal_width: 1.0,
            equilibration_steps: 1000,
            production_steps: 10000,
        }
    }
}

impl TemperatureController {
    pub fn new() -> Self {
        Self {
            control_method: TemperatureControlMethod::NoséHoover,
            thermostat_parameters: ThermostatParameters::new(),
            temperature_profile: TemperatureProfile::new(),
        }
    }

    /// Borrow the temperature control method.
    pub fn control_method(&self) -> &TemperatureControlMethod {
        &self.control_method
    }

    /// Set the temperature control method.
    pub fn set_control_method(&mut self, method: TemperatureControlMethod) {
        self.control_method = method;
    }

    /// Borrow the thermostat parameters.
    pub fn thermostat_parameters(&self) -> &ThermostatParameters {
        &self.thermostat_parameters
    }

    /// Mutably borrow the thermostat parameters.
    pub fn thermostat_parameters_mut(&mut self) -> &mut ThermostatParameters {
        &mut self.thermostat_parameters
    }

    /// Borrow the temperature profile.
    pub fn temperature_profile(&self) -> &TemperatureProfile {
        &self.temperature_profile
    }

    /// Mutably borrow the temperature profile.
    pub fn temperature_profile_mut(&mut self) -> &mut TemperatureProfile {
        &mut self.temperature_profile
    }

    pub fn initialize(&mut self) -> Result<(), ChemistryError> {
        Ok(())
    }
}

impl ThermostatParameters {
    pub fn new() -> Self {
        Self {
            coupling_constant: 1.0,
            relaxation_time: 100.0,
            damping_coefficient: 1.0,
        }
    }
}

impl TemperatureProfile {
    pub fn new() -> Self {
        Self {
            profile_type: TemperatureProfileType::Constant,
            initial_temperature: 300.0,
            final_temperature: None,
            ramp_rate: None,
        }
    }
}

impl ForceFieldCalculator {
    pub fn new() -> Self {
        Self {
            force_fields: HashMap::new(),
            interaction_calculator: InteractionCalculator::new(),
            energy_calculator: EnergyCalculator::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), ChemistryError> {
        self.interaction_calculator.initialize()?;
        self.energy_calculator.initialize()?;
        // Populate the standard force-field catalogue. Each entry ships with a
        // default parameter set (≥1 bond/angle/torsion/nonbonded entry) so the
        // calculator is usable immediately after initialization.
        self.register_standard_force_fields();
        Ok(())
    }

    /// Register the built-in force-field definitions (AMBER, CHARMM, OPLS-AA,
    /// GROMOS, Universal/UFF). `Universal` has no dedicated `ForceFieldType`
    /// variant, so it is tagged `Custom` with a descriptive name.
    fn register_standard_force_fields(&mut self) {
        self.force_fields.insert(
            "AMBER".to_string(),
            ForceField {
                field_id: "ff_amber".to_string(),
                field_name: "AMBER".to_string(),
                field_type: ForceFieldType::AMBER,
                parameters: ForceFieldParameters::new(),
            },
        );
        self.force_fields.insert(
            "CHARMM".to_string(),
            ForceField {
                field_id: "ff_charmm".to_string(),
                field_name: "CHARMM".to_string(),
                field_type: ForceFieldType::CHARMM,
                parameters: ForceFieldParameters::new(),
            },
        );
        self.force_fields.insert(
            "OPLS".to_string(),
            ForceField {
                field_id: "ff_opls".to_string(),
                field_name: "OPLS-AA".to_string(),
                field_type: ForceFieldType::OPLS,
                parameters: ForceFieldParameters::new(),
            },
        );
        self.force_fields.insert(
            "GROMOS".to_string(),
            ForceField {
                field_id: "ff_gromos".to_string(),
                field_name: "GROMOS".to_string(),
                field_type: ForceFieldType::GROMOS,
                parameters: ForceFieldParameters::new(),
            },
        );
        self.force_fields.insert(
            "Universal".to_string(),
            ForceField {
                field_id: "ff_uff".to_string(),
                field_name: "Universal (UFF)".to_string(),
                field_type: ForceFieldType::Custom,
                parameters: ForceFieldParameters::new(),
            },
        );
    }

    /// Look up a registered force field by name.
    pub fn get_force_field(&self, name: &str) -> Option<&ForceField> {
        self.force_fields.get(name)
    }

    /// List the names of all registered force fields.
    pub fn list_force_fields(&self) -> Vec<String> {
        self.force_fields.keys().cloned().collect()
    }

    /// Register a custom force field under `name`, replacing any existing entry.
    pub fn register_force_field(&mut self, name: &str, force_field: ForceField) {
        self.force_fields.insert(name.to_string(), force_field);
    }
}

impl ForceField {
    pub fn new() -> Self {
        Self {
            field_id: "ff_1".to_string(),
            field_name: "AMBER".to_string(),
            field_type: ForceFieldType::AMBER,
            parameters: ForceFieldParameters::new(),
        }
    }
}

impl ForceFieldParameters {
    pub fn new() -> Self {
        Self {
            bond_parameters: vec![BondParameter::new()],
            angle_parameters: vec![AngleParameter::new()],
            torsion_parameters: vec![TorsionParameter::new()],
            nonbonded_parameters: vec![NonbondedParameter::new()],
        }
    }
}

impl BondParameter {
    pub fn new() -> Self {
        Self {
            atom_types: vec!["C".to_string(), "H".to_string()],
            equilibrium_length: 1.09,
            force_constant: 450.0,
        }
    }
}

impl AngleParameter {
    pub fn new() -> Self {
        Self {
            atom_types: vec!["C".to_string(), "H".to_string(), "H".to_string()],
            equilibrium_angle: 109.5,
            force_constant: 50.0,
        }
    }
}

impl TorsionParameter {
    pub fn new() -> Self {
        Self {
            atom_types: vec![
                "C".to_string(),
                "C".to_string(),
                "C".to_string(),
                "C".to_string(),
            ],
            barriers: vec![0.0, 1.0],
            phases: vec![0.0, 180.0],
            periodicities: vec![1, 2],
        }
    }
}

impl NonbondedParameter {
    pub fn new() -> Self {
        Self {
            atom_type: "C".to_string(),
            sigma: 3.4,
            epsilon: 0.086,
            charge: 0.0,
        }
    }
}

impl InteractionCalculator {
    pub fn new() -> Self {
        Self {
            bonded_interactions: BondedInteractions::new(),
            nonbonded_interactions: NonbondedInteractions::new(),
            long_range_interactions: LongRangeInteractions::new(),
        }
    }

    /// Borrow the bonded-interactions calculator.
    pub fn bonded_interactions(&self) -> &BondedInteractions {
        &self.bonded_interactions
    }

    /// Mutably borrow the bonded-interactions calculator.
    pub fn bonded_interactions_mut(&mut self) -> &mut BondedInteractions {
        &mut self.bonded_interactions
    }

    /// Borrow the nonbonded-interactions calculator.
    pub fn nonbonded_interactions(&self) -> &NonbondedInteractions {
        &self.nonbonded_interactions
    }

    /// Mutably borrow the nonbonded-interactions calculator.
    pub fn nonbonded_interactions_mut(&mut self) -> &mut NonbondedInteractions {
        &mut self.nonbonded_interactions
    }

    /// Borrow the long-range-interactions calculator.
    pub fn long_range_interactions(&self) -> &LongRangeInteractions {
        &self.long_range_interactions
    }

    /// Mutably borrow the long-range-interactions calculator.
    pub fn long_range_interactions_mut(&mut self) -> &mut LongRangeInteractions {
        &mut self.long_range_interactions
    }

    pub fn initialize(&mut self) -> Result<(), ChemistryError> {
        let _ = self.nonbonded_interactions.lennard_jones();
        let _ = self.nonbonded_interactions.coulomb();
        let _ = self.nonbonded_interactions.buckingham();
        let _ = self.long_range_interactions.ewald_summation();
        let _ = self.long_range_interactions.particle_mesh();
        let _ = self.long_range_interactions.reaction_field();
        Ok(())
    }
}

impl BondedInteractions {
    pub fn new() -> Self {
        Self {
            bond_calculator: BondCalculator::new(),
            angle_calculator: AngleCalculator::new(),
            torsion_calculator: TorsionCalculator::new(),
            improper_calculator: ImproperCalculator::new(),
        }
    }

    /// Borrow the bond-stretch calculator.
    pub fn bond_calculator(&self) -> &BondCalculator {
        &self.bond_calculator
    }

    /// Mutably borrow the bond-stretch calculator.
    pub fn bond_calculator_mut(&mut self) -> &mut BondCalculator {
        &mut self.bond_calculator
    }

    /// Borrow the angle-bend calculator.
    pub fn angle_calculator(&self) -> &AngleCalculator {
        &self.angle_calculator
    }

    /// Mutably borrow the angle-bend calculator.
    pub fn angle_calculator_mut(&mut self) -> &mut AngleCalculator {
        &mut self.angle_calculator
    }

    /// Borrow the torsion calculator.
    pub fn torsion_calculator(&self) -> &TorsionCalculator {
        &self.torsion_calculator
    }

    /// Mutably borrow the torsion calculator.
    pub fn torsion_calculator_mut(&mut self) -> &mut TorsionCalculator {
        &mut self.torsion_calculator
    }

    /// Borrow the improper-torsion calculator.
    pub fn improper_calculator(&self) -> &ImproperCalculator {
        &self.improper_calculator
    }

    /// Mutably borrow the improper-torsion calculator.
    pub fn improper_calculator_mut(&mut self) -> &mut ImproperCalculator {
        &mut self.improper_calculator
    }
}

impl BondCalculator {
    pub fn new() -> Self {
        Self {
            calculator_type: BondCalculatorType::Harmonic,
            parameters: BondCalculatorParameters::new(),
        }
    }
}

impl BondCalculatorParameters {
    pub fn new() -> Self {
        Self {
            force_constant: 450.0,
            equilibrium_length: 1.09,
            dissociation_energy: None,
        }
    }
}

impl AngleCalculator {
    pub fn new() -> Self {
        Self {
            calculator_type: AngleCalculatorType::Harmonic,
            parameters: AngleCalculatorParameters::new(),
        }
    }
}

impl AngleCalculatorParameters {
    pub fn new() -> Self {
        Self {
            force_constant: 50.0,
            equilibrium_angle: 109.5,
            ub_parameters: None,
        }
    }
}

impl TorsionCalculator {
    pub fn new() -> Self {
        Self {
            calculator_type: TorsionCalculatorType::Cosine,
            parameters: TorsionCalculatorParameters::new(),
        }
    }
}

impl TorsionCalculatorParameters {
    pub fn new() -> Self {
        Self {
            barriers: vec![0.0, 1.0],
            phases: vec![0.0, 180.0],
            periodicities: vec![1, 2],
        }
    }
}

impl ImproperCalculator {
    pub fn new() -> Self {
        Self {
            calculator_type: ImproperCalculatorType::Harmonic,
            parameters: ImproperCalculatorParameters::new(),
        }
    }
}

impl ImproperCalculatorParameters {
    pub fn new() -> Self {
        Self {
            force_constant: 50.0,
            equilibrium_angle: 109.5,
        }
    }
}

impl NonbondedInteractions {
    pub fn new() -> Self {
        Self {
            lennard_jones: LennardJones::new(),
            coulomb: Coulomb::new(),
            buckingham: Buckingham::new(),
        }
    }

    /// Borrow the Lennard-Jones potential parameters.
    pub fn lennard_jones(&self) -> &LennardJones {
        &self.lennard_jones
    }

    /// Mutably borrow the Lennard-Jones potential parameters.
    pub fn lennard_jones_mut(&mut self) -> &mut LennardJones {
        &mut self.lennard_jones
    }

    /// Borrow the Coulomb potential parameters.
    pub fn coulomb(&self) -> &Coulomb {
        &self.coulomb
    }

    /// Mutably borrow the Coulomb potential parameters.
    pub fn coulomb_mut(&mut self) -> &mut Coulomb {
        &mut self.coulomb
    }

    /// Borrow the Buckingham potential parameters.
    pub fn buckingham(&self) -> &Buckingham {
        &self.buckingham
    }

    /// Mutably borrow the Buckingham potential parameters.
    pub fn buckingham_mut(&mut self) -> &mut Buckingham {
        &mut self.buckingham
    }
}

impl LennardJones {
    pub fn new() -> Self {
        Self {
            epsilon: 0.086,
            sigma: 3.4,
            cutoff: 12.0,
            switching_distance: 10.0,
        }
    }
}

impl Coulomb {
    pub fn new() -> Self {
        Self {
            coulomb_constant: 332.06,
            dielectric: 1.0,
            cutoff: 12.0,
            switching_distance: 10.0,
        }
    }
}

impl Buckingham {
    pub fn new() -> Self {
        Self {
            a: 1000.0,
            b: 3.5,
            c: 0.0,
            cutoff: 12.0,
        }
    }
}

impl LongRangeInteractions {
    pub fn new() -> Self {
        Self {
            ewald_summation: EwaldSummation::new(),
            particle_mesh: ParticleMesh::new(),
            reaction_field: ReactionField::new(),
        }
    }

    /// Borrow the Ewald-summation parameters.
    pub fn ewald_summation(&self) -> &EwaldSummation {
        &self.ewald_summation
    }

    /// Mutably borrow the Ewald-summation parameters.
    pub fn ewald_summation_mut(&mut self) -> &mut EwaldSummation {
        &mut self.ewald_summation
    }

    /// Borrow the particle-mesh (PME) parameters.
    pub fn particle_mesh(&self) -> &ParticleMesh {
        &self.particle_mesh
    }

    /// Mutably borrow the particle-mesh (PME) parameters.
    pub fn particle_mesh_mut(&mut self) -> &mut ParticleMesh {
        &mut self.particle_mesh
    }

    /// Borrow the reaction-field parameters.
    pub fn reaction_field(&self) -> &ReactionField {
        &self.reaction_field
    }

    /// Mutably borrow the reaction-field parameters.
    pub fn reaction_field_mut(&mut self) -> &mut ReactionField {
        &mut self.reaction_field
    }
}

impl EwaldSummation {
    pub fn new() -> Self {
        Self {
            alpha: 0.3,
            k_max: 10,
            real_cutoff: 12.0,
            reciprocal_cutoff: 10.0,
        }
    }
}

impl ParticleMesh {
    pub fn new() -> Self {
        Self {
            grid_size: vec![32, 32, 32],
            spline_order: 4,
            cutoff: 12.0,
        }
    }
}

impl ReactionField {
    pub fn new() -> Self {
        Self {
            dielectric_inside: 1.0,
            dielectric_outside: 78.5,
            cutoff: 12.0,
        }
    }
}

impl EnergyCalculator {
    pub fn new() -> Self {
        Self {
            kinetic_energy: KineticEnergy::new(),
            potential_energy: PotentialEnergy::new(),
            total_energy: TotalEnergy::new(),
        }
    }

    /// Borrow the kinetic-energy state.
    pub fn kinetic_energy(&self) -> &KineticEnergy {
        &self.kinetic_energy
    }

    /// Mutably borrow the kinetic-energy state.
    pub fn kinetic_energy_mut(&mut self) -> &mut KineticEnergy {
        &mut self.kinetic_energy
    }

    /// Borrow the potential-energy state.
    pub fn potential_energy(&self) -> &PotentialEnergy {
        &self.potential_energy
    }

    /// Mutably borrow the potential-energy state.
    pub fn potential_energy_mut(&mut self) -> &mut PotentialEnergy {
        &mut self.potential_energy
    }

    /// Borrow the total-energy state.
    pub fn total_energy(&self) -> &TotalEnergy {
        &self.total_energy
    }

    /// Mutably borrow the total-energy state.
    pub fn total_energy_mut(&mut self) -> &mut TotalEnergy {
        &mut self.total_energy
    }

    pub fn initialize(&mut self) -> Result<(), ChemistryError> {
        Ok(())
    }
}

impl KineticEnergy {
    pub fn new() -> Self {
        Self {
            temperature: 300.0,
            degrees_of_freedom: 0,
            velocities: Vec::new(),
        }
    }
}

impl PotentialEnergy {
    pub fn new() -> Self {
        Self {
            bonded_energy: 0.0,
            nonbonded_energy: 0.0,
            long_range_energy: 0.0,
        }
    }
}

impl TotalEnergy {
    pub fn new() -> Self {
        Self {
            kinetic: 0.0,
            potential: 0.0,
            total: 0.0,
            drift: 0.0,
        }
    }
}

impl MolecularIntegrator {
    pub fn new() -> Self {
        Self {
            integrator_type: IntegratorType::VelocityVerlet,
            integrator_parameters: IntegratorParameters::new(),
            constraint_handler: ConstraintHandler::new(),
        }
    }

    /// Borrow the integrator type.
    pub fn integrator_type(&self) -> &IntegratorType {
        &self.integrator_type
    }

    /// Set the integrator type.
    pub fn set_integrator_type(&mut self, integrator_type: IntegratorType) {
        self.integrator_type = integrator_type;
    }

    /// Borrow the integrator parameters.
    pub fn integrator_parameters(&self) -> &IntegratorParameters {
        &self.integrator_parameters
    }

    /// Mutably borrow the integrator parameters.
    pub fn integrator_parameters_mut(&mut self) -> &mut IntegratorParameters {
        &mut self.integrator_parameters
    }

    pub fn initialize(&mut self) -> Result<(), ChemistryError> {
        self.constraint_handler.initialize()?;
        Ok(())
    }
}

impl IntegratorParameters {
    pub fn new() -> Self {
        Self {
            time_step: 0.001,
            accuracy: 1e-6,
            stability_factor: 0.9,
        }
    }
}

impl ConstraintHandler {
    pub fn new() -> Self {
        Self {
            constraint_algorithm: ConstraintAlgorithm::SHAKE,
            constraint_parameters: ConstraintParameters::new(),
        }
    }

    /// Borrow the constraint algorithm.
    pub fn constraint_algorithm(&self) -> &ConstraintAlgorithm {
        &self.constraint_algorithm
    }

    /// Set the constraint algorithm.
    pub fn set_constraint_algorithm(&mut self, algorithm: ConstraintAlgorithm) {
        self.constraint_algorithm = algorithm;
    }

    /// Borrow the constraint parameters.
    pub fn constraint_parameters(&self) -> &ConstraintParameters {
        &self.constraint_parameters
    }

    /// Mutably borrow the constraint parameters.
    pub fn constraint_parameters_mut(&mut self) -> &mut ConstraintParameters {
        &mut self.constraint_parameters
    }

    pub fn initialize(&mut self) -> Result<(), ChemistryError> {
        Ok(())
    }
}

impl ConstraintParameters {
    pub fn new() -> Self {
        Self {
            tolerance: 1e-6,
            max_iterations: 100,
            relaxation_parameter: 0.1,
        }
    }
}

impl BoundaryConditions {
    pub fn new() -> Self {
        Self {
            boundary_type: BoundaryType::Periodic,
            box_vectors: vec![
                vec![10.0, 0.0, 0.0],
                vec![0.0, 10.0, 0.0],
                vec![0.0, 0.0, 10.0],
            ],
            minimum_image: MinimumImage::new(),
        }
    }

    /// Borrow the boundary type.
    pub fn boundary_type(&self) -> &BoundaryType {
        &self.boundary_type
    }

    /// Set the boundary type.
    pub fn set_boundary_type(&mut self, boundary_type: BoundaryType) {
        self.boundary_type = boundary_type;
    }

    /// Borrow the simulation box vectors.
    pub fn box_vectors(&self) -> &Vec<Vec<f64>> {
        &self.box_vectors
    }

    /// Mutably borrow the simulation box vectors.
    pub fn box_vectors_mut(&mut self) -> &mut Vec<Vec<f64>> {
        &mut self.box_vectors
    }

    /// Borrow the minimum-image convention state.
    pub fn minimum_image(&self) -> &MinimumImage {
        &self.minimum_image
    }

    /// Mutably borrow the minimum-image convention state.
    pub fn minimum_image_mut(&mut self) -> &mut MinimumImage {
        &mut self.minimum_image
    }
}

impl MinimumImage {
    pub fn new() -> Self {
        Self {
            box_size: vec![10.0, 10.0, 10.0],
            periodic: true,
        }
    }
}
