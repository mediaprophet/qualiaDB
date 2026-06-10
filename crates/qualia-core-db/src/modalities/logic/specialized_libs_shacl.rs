//! SHACL Extensions for Specialized Libraries
//!
//! This module provides SHACL constraint extensions for all specialized libraries
//! including linear algebra, machine learning, physics simulation, chemistry modeling,
//! medical computing, financial modeling, engineering analysis, and statistical computing.

use crate::webizen::SlgOpcode;

// ── Linear Algebra Constraints ───────────────────────────────────────────────

/// `q42:MatrixConfiguration` — validates matrix storage and computation configuration
#[derive(Debug, Clone)]
pub struct MatrixConfiguration {
    pub max_matrix_size: u64,        // Maximum matrix dimension
    pub max_zone_capacity: u64,      // Maximum zone capacity in bytes
    pub allowed_zone_types: Vec<String>, // ["Dense", "Sparse", "Structured", "Temporary"]
    pub require_zero_copy: bool,     // Require zero-copy operations
}

/// `q42:MatrixOperation` — validates matrix operation parameters
#[derive(Debug, Clone)]
pub struct MatrixOperation {
    pub operation_type: String,     // "multiply", "add", "subtract", "invert", etc.
    pub max_condition_number: f64,  // Maximum allowed condition number
    pub require_numerical_stability: bool,
    pub precision_mode: String,     // "f32", "f64", "f128"
}

/// `q42:EigenDecomposition` — validates eigenvalue decomposition parameters
#[derive(Debug, Clone)]
pub struct EigenDecomposition {
    pub max_iterations: u32,
    pub convergence_tolerance: f64,
    pub require_hermitian: bool,    // For real eigenvalues
    pub algorithm_type: String,     // "qr", "power", "jacobi"
}

// ── Machine Learning Constraints ─────────────────────────────────────────────

/// `q42:ModelConfiguration` — validates ML model configuration
#[derive(Debug, Clone)]
pub struct ModelConfiguration {
    pub max_model_size_mb: u64,     // Maximum model size in MB
    pub max_parameters: u64,        // Maximum number of parameters
    pub allowed_model_types: Vec<String>, // ["neural_network", "decision_tree", "svm", etc.]
    pub require_quantization: bool, // Require model quantization
}

/// `q42:TrainingConfiguration` — validates training hyperparameters
#[derive(Debug, Clone)]
pub struct TrainingConfiguration {
    pub max_epochs: u32,
    pub batch_size_range: (u32, u32),
    pub learning_rate_range: (f64, f64),
    pub allowed_optimizers: Vec<String>, // ["adam", "sgd", "rmsprop", etc.]
    pub require_early_stopping: bool,
}

/// `q42:InferenceConfiguration` — validates inference parameters
#[derive(Debug, Clone)]
pub struct InferenceConfiguration {
    pub max_batch_size: u32,
    pub max_latency_ms: u64,
    pub allowed_precision_modes: Vec<String>, // ["fp32", "fp16", "int8"]
    pub require_batch_normalization: bool,
}

// ── Physics Simulation Constraints ───────────────────────────────────────────

/// `q42:SimulationConfiguration` — validates physics simulation configuration
#[derive(Debug, Clone)]
pub struct SimulationConfiguration {
    pub max_time_steps: u64,
    pub max_spatial_resolution: u32,
    pub allowed_time_integrators: Vec<String>, // ["euler", "runge_kutta", "verlet"]
    pub require_energy_conservation: bool,
    pub max_cfl_number: f64,       // Courant-Friedrichs-Lewy condition
}

/// `q42:BoundaryConditions` — validates boundary condition parameters
#[derive(Debug, Clone)]
pub struct BoundaryConditions {
    pub allowed_boundary_types: Vec<String>, // ["dirichlet", "neumann", "periodic", "mixed"]
    pub require_consistency: bool,
    pub max_gradient: f64,
}

/// `q42:MeshConfiguration` — validates mesh generation parameters
#[derive(Debug, Clone)]
pub struct MeshConfiguration {
    pub max_elements: u64,
    pub min_element_quality: f64,  // Aspect ratio, skewness, etc.
    pub allowed_element_types: Vec<String>, // ["triangle", "quad", "tetrahedron", "hexahedron"]
    pub require_manifold: bool,
}

// ── Chemistry Modeling Constraints ───────────────────────────────────────────

/// `q42:MoleculeConfiguration` — validates molecular structure configuration
#[derive(Debug, Clone)]
pub struct MoleculeConfiguration {
    pub max_atoms: u32,
    pub max_bonds: u32,
    pub allowed_element_types: Vec<String>, // Periodic table symbols
    pub require_valence_satisfaction: bool,
}

/// `q42:ReactionConfiguration` — validates chemical reaction parameters
#[derive(Debug, Clone)]
pub struct ReactionConfiguration {
    pub max_reactants: u32,
    pub max_products: u32,
    pub require_mass_balance: bool,
    pub require_charge_balance: bool,
    pub allowed_reaction_types: Vec<String>, // ["synthesis", "decomposition", "redox"]
}

/// `q42:QuantumCalculation` — validates quantum chemistry calculation parameters
#[derive(Debug, Clone)]
pub struct QuantumCalculation {
    pub max_basis_functions: u32,
    pub allowed_methods: Vec<String>, // ["dft", "hf", "mp2", "ccsd"]
    pub max_scf_iterations: u32,
    pub convergence_threshold: f64,
}

// ── Medical Computing Constraints ─────────────────────────────────────────────

/// `q42:MedicalDataConfiguration` — validates medical data parameters
#[derive(Debug, Clone)]
pub struct MedicalDataConfiguration {
    pub require_hipaa_compliance: bool,
    pub require_de_identification: bool,
    pub allowed_data_types: Vec<String>, // ["fhir", "dicom", "hl7"]
    pub max_patient_records: u64,
}

/// `q42:ClinicalDecisionConfiguration` — validates clinical decision support parameters
#[derive(Debug, Clone)]
pub struct ClinicalDecisionConfiguration {
    pub require_evidence_based: bool,
    pub max_confidence_interval: f64,
    pub allowed_decision_types: Vec<String>, // ["diagnosis", "treatment", "prognosis"]
    pub require_physician_review: bool,
}

/// `q42:MedicalImagingConfiguration` — validates medical imaging parameters
#[derive(Debug, Clone)]
pub struct MedicalImagingConfiguration {
    pub allowed_modalities: Vec<String>, // ["mri", "ct", "xray", "ultrasound"]
    pub max_resolution: (u32, u32),    // (width, height)
    pub require_dicom_compliance: bool,
    pub max_file_size_mb: u64,
}

// ── Financial Modeling Constraints ───────────────────────────────────────────

/// `q42:FinancialModelConfiguration` — validates financial model parameters
#[derive(Debug, Clone)]
pub struct FinancialModelConfiguration {
    pub max_time_horizon_days: u32,
    pub allowed_asset_classes: Vec<String>, // ["equity", "fixed_income", "derivative", "crypto"]
    pub require_risk_metrics: bool,
    pub max_leverage_ratio: f64,
}

/// `q42:RiskCalculation` — validates risk calculation parameters
#[derive(Debug, Clone)]
pub struct RiskCalculation {
    pub allowed_risk_models: Vec<String>, // ["var", "cvar", "expected_shortfall"]
    pub confidence_level_range: (f64, f64), // e.g., (0.95, 0.99)
    pub max_lookback_days: u32,
    pub require_stress_testing: bool,
}

/// `q42:TradingConfiguration` — validates trading strategy parameters
#[derive(Debug, Clone)]
pub struct TradingConfiguration {
    pub max_position_size: f64,
    pub allowed_order_types: Vec<String>, // ["market", "limit", "stop", "stop_limit"]
    pub require_risk_limits: bool,
    pub max_daily_trades: u32,
}

// ── Engineering Analysis Constraints ─────────────────────────────────────────

/// `q42:EngineeringSimulationConfiguration` — validates engineering simulation parameters
#[derive(Debug, Clone)]
pub struct EngineeringSimulationConfiguration {
    pub max_mesh_elements: u64,
    pub allowed_analysis_types: Vec<String>, // ["structural", "thermal", "fluid", "electromagnetic"]
    pub require_convergence: bool,
    pub max_simulation_time_hours: f64,
}

/// `q42:MaterialProperties` — validates material property parameters
#[derive(Debug, Clone)]
pub struct MaterialProperties {
    pub allowed_material_types: Vec<String>, // ["metal", "polymer", "ceramic", "composite"]
    pub require_standard_compliance: bool, // ASTM, ISO, etc.
    pub max_temperature_kelvin: f64,
    pub min_safety_factor: f64,
}

/// `q42:LoadConfiguration` — validates load and boundary condition parameters
#[derive(Debug, Clone)]
pub struct LoadConfiguration {
    pub max_load_magnitude: f64,
    pub allowed_load_types: Vec<String>, // ["static", "dynamic", "thermal", "electromagnetic"]
    pub require_load_combination: bool,
    pub safety_factor_range: (f64, f64),
}

// ── Statistical Computing Constraints ───────────────────────────────────────

/// `q42:StatisticalAnalysisConfiguration` — validates statistical analysis parameters
#[derive(Debug, Clone)]
pub struct StatisticalAnalysisConfiguration {
    pub max_sample_size: u64,
    pub allowed_test_types: Vec<String>, // ["t_test", "anova", "chi_square", "regression"]
    pub require_normality_test: bool,
    pub significance_level_range: (f64, f64),
}

/// `q42:DistributionConfiguration` — validates probability distribution parameters
#[derive(Debug, Clone)]
pub struct DistributionConfiguration {
    pub allowed_distributions: Vec<String>, // ["normal", "binomial", "poisson", "exponential"]
    pub require_parameter_constraints: bool,
    pub max_mixture_components: u32,
}

/// `q42:SamplingConfiguration` — validates sampling method parameters
#[derive(Debug, Clone)]
pub struct SamplingConfiguration {
    pub allowed_sampling_methods: Vec<String>, // ["monte_carlo", "bootstrap", "jackknife"]
    pub max_iterations: u32,
    pub require_convergence_diagnostics: bool,
}

// ── Cryptographic Library Constraints ─────────────────────────────────────────

/// `q42:CryptographicConfiguration` — validates cryptographic operation parameters
#[derive(Debug, Clone)]
pub struct CryptographicConfiguration {
    pub min_key_length_bits: u16,
    pub allowed_algorithms: Vec<String>, // ["aes", "rsa", "ecc", "sha256"]
    pub require_fips_compliance: bool,
    pub max_operation_time_ms: u64,
}

/// `q42:KeyManagementConfiguration` — validates key management parameters
#[derive(Debug, Clone)]
pub struct KeyManagementConfiguration {
    pub require_hsm: bool,                // Hardware Security Module
    pub allowed_key_types: Vec<String>, // ["symmetric", "asymmetric", "hash"]
    pub max_key_lifetime_days: u32,
    pub require_key_rotation: bool,
}

/// `q42:DigitalSignatureConfiguration` — validates digital signature parameters
#[derive(Debug, Clone)]
pub struct DigitalSignatureConfiguration {
    pub allowed_signature_algorithms: Vec<String>, // ["ed25519", "rsa_pss", "ecdsa"]
    pub require_timestamp: bool,
    pub max_signature_size_bytes: u32,
}

// ── QPU Bridge Constraints ─────────────────────────────────────────────────────

/// `q42:QPUConfiguration` — validates quantum processing unit parameters
#[derive(Debug, Clone)]
pub struct QPUConfiguration {
    pub max_qubits: u16,
    pub allowed_qpu_types: Vec<String>, // ["dwave", "ibm", "google", "rigetti"]
    pub max_circuit_depth: u32,
    pub require_error_correction: bool,
}

/// `q42:QuantumCircuitConfiguration` — validates quantum circuit parameters
#[derive(Debug, Clone)]
pub struct QuantumCircuitConfiguration {
    pub max_gates: u32,
    pub allowed_gate_types: Vec<String>, // ["hadamard", "cnot", "phase", "measurement"]
    pub require_compilation: bool,
    pub max_execution_time_ms: u64,
}

/// `q42:QuantumAnnealingConfiguration` — validates quantum annealing parameters
#[derive(Debug, Clone)]
pub struct QuantumAnnealingConfiguration {
    pub max_annealing_time_us: u32,
    pub allowed_anneal_schedules: Vec<String>, // ["linear", "reverse", "custom"]
    pub require_ground_state_verification: bool,
}

// ── Quantum Biology Constraints ───────────────────────────────────────────────

/// `q42:BiomolecularConfiguration` — validates biomolecular simulation parameters
#[derive(Debug, Clone)]
pub struct BiomolecularConfiguration {
    pub max_atoms: u32,
    pub max_residues: u32,
    pub allowed_force_fields: Vec<String>, // ["amber", "charmm", "opls"]
    pub max_simulation_time_ns: f64,
}

/// `q42:QuantumBiologyCalculation` — validates quantum biology calculation parameters
#[derive(Debug, Clone)]
pub struct QuantumBiologyCalculation {
    pub max_quantum_states: u32,
    pub allowed_methods: Vec<String>, // ["dft", "semi_empirical", "force_field"]
    pub max_convergence_iterations: u32,
    require_solvent_model: bool,
}

// ── Opcode Generation Functions ───────────────────────────────────────────────

impl MatrixConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_matrix_size as f64),
            SlgOpcode::CheckMaxInclusive(self.max_zone_capacity as f64),
        ]
    }
}

impl MatrixOperation {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_condition_number),
            SlgOpcode::CheckHasValue(crate::q_hash(&self.precision_mode)),
        ]
    }
}

impl ModelConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_model_size_mb as f64),
            SlgOpcode::CheckMaxInclusive(self.max_parameters as f64),
        ]
    }
}

impl TrainingConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_epochs as f64),
            SlgOpcode::CheckMinInclusive(self.batch_size_range.0 as f64),
            SlgOpcode::CheckMaxInclusive(self.batch_size_range.1 as f64),
            SlgOpcode::CheckMinInclusive(self.learning_rate_range.0),
            SlgOpcode::CheckMaxInclusive(self.learning_rate_range.1),
        ]
    }
}

impl SimulationConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_time_steps as f64),
            SlgOpcode::CheckMaxInclusive(self.max_spatial_resolution as f64),
            SlgOpcode::CheckMaxInclusive(self.max_cfl_number),
        ]
    }
}

impl CryptographicConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMinInclusive(self.min_key_length_bits as f64),
            SlgOpcode::CheckMaxInclusive(self.max_operation_time_ms as f64),
        ]
    }
}

impl QPUConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_qubits as f64),
            SlgOpcode::CheckMaxInclusive(self.max_circuit_depth as f64),
        ]
    }
}

// Generic opcode generation for other constraint types
macro_rules! generate_simple_opcodes {
    ($struct_name:ident, $field:ident) => {
        impl $struct_name {
            pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
                vec![SlgOpcode::CheckMaxInclusive(self.$field as f64)]
            }
        }
    };
}

generate_simple_opcodes!(EigenDecomposition, max_iterations);
generate_simple_opcodes!(InferenceConfiguration, max_batch_size);
generate_simple_opcodes!(BoundaryConditions, max_gradient);
generate_simple_opcodes!(MeshConfiguration, max_elements);
generate_simple_opcodes!(MoleculeConfiguration, max_atoms);
generate_simple_opcodes!(ReactionConfiguration, max_reactants);
generate_simple_opcodes!(QuantumCalculation, max_basis_functions);
generate_simple_opcodes!(MedicalDataConfiguration, max_patient_records);
generate_simple_opcodes!(MedicalImagingConfiguration, max_file_size_mb);
generate_simple_opcodes!(FinancialModelConfiguration, max_time_horizon_days);
generate_simple_opcodes!(RiskCalculation, max_lookback_days);
generate_simple_opcodes!(TradingConfiguration, max_daily_trades);
generate_simple_opcodes!(EngineeringSimulationConfiguration, max_mesh_elements);
generate_simple_opcodes!(MaterialProperties, max_temperature_kelvin);
generate_simple_opcodes!(LoadConfiguration, max_load_magnitude);
generate_simple_opcodes!(StatisticalAnalysisConfiguration, max_sample_size);
generate_simple_opcodes!(DistributionConfiguration, max_mixture_components);
generate_simple_opcodes!(SamplingConfiguration, max_iterations);
generate_simple_opcodes!(KeyManagementConfiguration, max_key_lifetime_days);
generate_simple_opcodes!(DigitalSignatureConfiguration, max_signature_size_bytes);
generate_simple_opcodes!(QuantumCircuitConfiguration, max_gates);
generate_simple_opcodes!(QuantumAnnealingConfiguration, max_annealing_time_us);
generate_simple_opcodes!(BiomolecularConfiguration, max_atoms);
generate_simple_opcodes!(QuantumBiologyCalculation, max_quantum_states);

// ── SHACL TTL Vocabulary for Specialized Libraries ─────────────────────────────

/// Returns comprehensive SHACL TTL vocabulary for all specialized libraries
pub fn get_specialized_libs_shacl_ttl() -> &'static str {
    r#"
@prefix q42: <https://qualia.network/q42#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

# ── Linear Algebra Constraints ─────────────────────────────────────────────

q42:MatrixConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxMatrixSize ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 1000000 ;
        sh:message "Matrix size must be between 1 and 1,000,000" ;
    ] ;
    sh:property [
        sh:path q42:maxZoneCapacity ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1024 ;
        sh:maxInclusive 1099511627776 ;
        sh:message "Zone capacity must be between 1KB and 1TB" ;
    ] .

q42:MatrixOperationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:operationType ;
        sh:in ("multiply" "add" "subtract" "invert" "transpose" "decompose") ;
        sh:message "Operation type must be a valid matrix operation" ;
    ] ;
    sh:property [
        sh:path q42:maxConditionNumber ;
        sh:datatype xsd:float ;
        sh:minInclusive 1.0 ;
        sh:maxInclusive 1e15 ;
        sh:message "Condition number must be reasonable for numerical stability" ;
    ] .

# ── Machine Learning Constraints ─────────────────────────────────────────────

q42:ModelConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxModelSizeMb ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 100000 ;
        sh:message "Model size must be between 1MB and 100GB" ;
    ] ;
    sh:property [
        sh:path q42:allowedModelTypes ;
        sh:in ("neural_network" "decision_tree" "svm" "random_forest" "knn") ;
        sh:message "Model type must be supported" ;
    ] .

q42:TrainingConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxEpochs ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 10000 ;
        sh:message "Training epochs must be between 1 and 10,000" ;
    ] ;
    sh:property [
        sh:path q42:learningRateRange ;
        sh:datatype xsd:float ;
        sh:minInclusive 0.0 ;
        sh:maxInclusive 1.0 ;
        sh:message "Learning rate must be between 0 and 1" ;
    ] .

# ── Physics Simulation Constraints ───────────────────────────────────────────

q42:SimulationConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxTimeSteps ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 1000000000 ;
        sh:message "Time steps must be between 1 and 1 billion" ;
    ] ;
    sh:property [
        sh:path q42:maxCflNumber ;
        sh:datatype xsd:float ;
        sh:minInclusive 0.0 ;
        sh:maxInclusive 1.0 ;
        sh:message "CFL number must be ≤ 1.0 for stability" ;
    ] .

# ── Chemistry Modeling Constraints ───────────────────────────────────────────

q42:MoleculeConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxAtoms ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 10000 ;
        sh:message "Molecule must have between 1 and 10,000 atoms" ;
    ] ;
    sh:property [
        sh:path q42:allowedElementTypes ;
        sh:message "Element types must be valid periodic table symbols" ;
    ] .

# ── Medical Computing Constraints ─────────────────────────────────────────────

q42:MedicalDataConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:requireHipaaCompliance ;
        sh:datatype xsd:boolean ;
        sh:message "HIPAA compliance flag must be boolean" ;
    ] ;
    sh:property [
        sh:path q42:allowedDataTypes ;
        sh:in ("fhir" "dicom" "hl7" "cda") ;
        sh:message "Data type must be a supported medical format" ;
    ] .

# ── Financial Modeling Constraints ───────────────────────────────────────────

q42:FinancialModelConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxTimeHorizonDays ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 36500 ;
        sh:message "Time horizon must be between 1 day and 100 years" ;
    ] ;
    sh:property [
        sh:path q42:maxLeverageRatio ;
        sh:datatype xsd:float ;
        sh:minInclusive 0.0 ;
        sh:maxInclusive 100.0 ;
        sh:message "Leverage ratio must be between 0 and 100" ;
    ] .

# ── Cryptographic Constraints ─────────────────────────────────────────────────

q42:CryptographicConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:minKeyLengthBits ;
        sh:datatype xsd:integer ;
        sh:minInclusive 128 ;
        sh:maxInclusive 4096 ;
        sh:message "Key length must be between 128 and 4096 bits" ;
    ] ;
    sh:property [
        sh:path q42:allowedAlgorithms ;
        sh:in ("aes" "rsa" "ecc" "sha256" "sha512" "ed25519") ;
        sh:message "Algorithm must be supported" ;
    ] .

# ── QPU Bridge Constraints ─────────────────────────────────────────────────────

q42:QPUConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxQubits ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 10000 ;
        sh:message "QPU must support between 1 and 10,000 qubits" ;
    ] ;
    sh:property [
        sh:path q42:allowedQpuTypes ;
        sh:in ("dwave" "ibm" "google" "rigetti" "ionq") ;
        sh:message "QPU type must be supported" ;
    ] .
"#
}
