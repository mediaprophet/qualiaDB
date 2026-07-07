//! Logic Modality
//!
//! This module contains all logic-related modalities for the QualiaDB engine.
//! It includes deontic logic, core logic evaluation, QUBO compilation,
//! N3 rule processing, SHACL constraint compilation, and OWL conversion.

// ─── Core Logic ─────────────────────────────────────────────────────────────

pub mod core;
pub use core::{WebizenCompiler, WebizenOpcode, WebizenVM};

// ─── Deontic Logic ───────────────────────────────────────────────────────────

pub mod deontic;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use deontic::compile_n3_rule_to_norm;
pub use deontic::{
    compile_norm_quin, defeater_fingerprint, evaluate_deontic_contract, extract_deontic_opcode,
    extract_expiry_unix32, harvest_defeater_fingerprints, norm_has_active_defeater, DeonticError,
    DeonticStatus, DeonticVerdict, DEFEATER_BIT, MAX_DEFEATER_SLOTS, OP_FORBID, OP_OBLIGATE,
    OP_PERMIT,
};

// ─── QUBO Compilation ─────────────────────────────────────────────────────────

pub mod qubo;

// ─── N3 Rule Processing ────────────────────────────────────────────────────────

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod n3_compiler;
pub mod n3_parser;
pub mod n3logic;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use n3_compiler::{
    validate_rule_against_shapes, AgentIntentFrame, N3CompileError, N3CompiledProgram,
    N3OutputMode, SentinelError, MAX_CONTEXT_NAMESPACE_SLOTS, MAX_INTENT_SCOPE_SLOTS,
};
pub use n3_parser::{N3Event, N3Parser, Rule, RuleType, Term};

// ─── SHACL Constraint Compilation ───────────────────────────────────────────────

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod shacl;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use shacl::{
    CalcComputeTarget, ClinicalRiskModel, CompiledShape, NodeKindType, PropertyPath,
    ProteinScoringMatrix, ShaclCompiler, ShaclConstraint, ShaclSeverity,
};

// ─── SHACL Extensions for New Client Features ───────────────────────────────────

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod shacl_extensions;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use shacl_extensions::{
    LogConfiguration, LogEntry, LogExportFormat, LogLevel, LogRetention, NetworkConfiguration,
    SecurityConfiguration, StorageConfiguration, SystemTrayConfiguration,
    TaxRecipientConfiguration, TrayAction, TrayMenuItem, TrayStatusIndicator,
};

// ─── SHACL Extensions for Specialized Libraries ─────────────────────────────────

pub mod geometry_asset_shacl;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod specialized_libs_shacl;
pub use geometry_asset_shacl::{
    validate_geometry_manifest, GeometryAssetConfiguration, GeometryConstraintViolation,
    GeometryManifestFacts, MAX_GEOMETRY_COUNT,
};
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use specialized_libs_shacl::{
    // Quantum Biology
    BiomolecularConfiguration,
    BoundaryConditions,
    ClinicalDecisionConfiguration,
    // Cryptographic Library
    CryptographicConfiguration,
    DeterminantConfiguration,
    DigitalSignatureConfiguration,
    DistributionConfiguration,
    EigenDecomposition,
    // Engineering Analysis
    EngineeringSimulationConfiguration,
    // Financial Modeling
    FinancialModelConfiguration,
    InferenceConfiguration,
    KeyManagementConfiguration,
    LoadConfiguration,
    MaterialProperties,
    // Linear Algebra
    MatrixConfiguration,
    MatrixOperation,
    // Medical Computing
    MedicalDataConfiguration,
    MedicalImagingConfiguration,
    MeshConfiguration,
    // Machine Learning
    ModelConfiguration,
    // Chemistry Modeling
    MoleculeConfiguration,
    PolynomialSolveConfiguration,
    // QPU Bridge
    QPUConfiguration,
    QuantumAnnealingConfiguration,
    QuantumBiologyCalculation,
    QuantumCalculation,
    QuantumCircuitConfiguration,
    ReactionConfiguration,
    RiskCalculation,
    SamplingConfiguration,
    // Physics Simulation
    SimulationConfiguration,
    // Statistical Computing
    StatisticalAnalysisConfiguration,
    SvdConfiguration,
    SymbolicExpressionConfiguration,
    SymbolicOperationConfiguration,
    TradingConfiguration,
    TrainingConfiguration,
};

// ─── SHACL Extensions for the Computational-Mathematics Engine ──────────────────

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod computational_maths_shacl;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use computational_maths_shacl::{
    get_computational_maths_shacl_ttl, AssumptionConfiguration, ExactArithmeticConfiguration,
    IntegralTransformConfiguration, InterpolationConfiguration, NumberTheoryConfiguration,
    NumericalMethodConfiguration, SpecialFunctionConfiguration, SymbolicCalculusConfiguration,
    UnitsConfiguration, VectorCalculusConfiguration, COMPUTATIONAL_MATHS_SHAPES,
};

// ─── SHACL Extensions for Core Modalities ───────────────────────────────────────

pub mod logic_modalities_shacl;
pub use logic_modalities_shacl::{get_logic_modalities_shacl_ttl, LOGIC_MODALITY_SHAPES};

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod core_modalities_shacl;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use core_modalities_shacl::{
    // ASP
    ASPConfiguration,
    AllenIntervalConfiguration,
    ArgumentEvaluationConfiguration,
    // Argumentation
    ArgumentationConfiguration,
    BayesianInferenceConfiguration,
    // Calculus
    CalculusConfiguration,
    ContradictionHandling,
    // Control Feedback
    ControlFeedbackConfiguration,
    // DL
    DLConfiguration,
    DLQueryConfiguration,
    // Dialectical Logic
    DialecticalConfiguration,
    // Diffusion
    DiffusionConfiguration,
    DiffusionGridConfiguration,
    // Epistemic Logic
    EpistemicConfiguration,
    EpistemicQuery,
    FeedbackGainConfiguration,
    GraphAlgorithmConfiguration,
    GraphAnalysisConfiguration,
    // Graph Theory
    GraphConfiguration,
    // Interval Reasoning
    IntervalArithmeticConfiguration,
    // Temporal LTL
    LTLConfiguration,
    // Linear Logic
    LinearLogicConfiguration,
    ODEConfiguration,
    // Paraconsistent Logic
    ParaconsistentConfiguration,
    // Probabilistic
    ProbabilisticConfiguration,
    SpatialRegionConfiguration,
    // Spatio-Temporal
    SpatioTemporalConfiguration,
    StableModelConfiguration,
    SynthesisConfiguration,
    TemporalTrace,
    TensorProvenanceConfiguration,
};

// ─── SHACL Extensions for Infrastructure ───────────────────────────────────────

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod infrastructure_shacl;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use infrastructure_shacl::{
    // Domain-Specific
    BiologicalDomainConfiguration,
    CalculusSolverConfiguration,
    ChemicalDomainConfiguration,
    DomainTransformerConfiguration,
    FinancialDomainConfiguration,
    // Geometric Algebra
    GeometricAlgebraConfiguration,
    GeospatialDomainConfiguration,
    HybridStateConfiguration,
    LinearAlgebraSolverConfiguration,
    MathematicalDomainConfiguration,
    // Obfuscation
    ObfuscationConfiguration,
    OptimizationSolverConfiguration,
    PhysicalDomainConfiguration,
    PolynomialObfuscationConfiguration,
    QuantumOptimizerConfiguration,
    SemanticStripperConfiguration,
    // Solvers
    SolverConfiguration,
    SymbolicLogicSolverConfiguration,
};

// ─── OWL Conversion ───────────────────────────────────────────────────────────

pub mod owl;

// ─── Rules ─────────────────────────────────────────────────────────────────────

pub mod rules;
pub use rules::{RuleEngine, RuleSet, GUARDIANSHIP_RULESET};

// ─── Opcodes ─────────────────────────────────────────────────────────────────

/// Epistemic logic opcodes (0x20-0x22)
pub const OP_KNOW: u8 = 0x20;
pub const OP_BELIEVE: u8 = 0x21;
pub const OP_DOUBT: u8 = 0x22;

/// Paraconsistent logic opcodes (0x30-0x32)
pub const OP_CONTRADICTION: u8 = 0x30;
pub const OP_GLUT: u8 = 0x31;
pub const OP_RELEVANCE: u8 = 0x32;

/// LTL (Linear Temporal Logic) opcodes (0x40-0x44)
pub const OP_NEXT: u8 = 0x40;
pub const OP_UNTIL: u8 = 0x41;
pub const OP_ALWAYS: u8 = 0x42;
pub const OP_EVENTUALLY: u8 = 0x43;
pub const OP_RELEASE: u8 = 0x44;

// ─── Re-exports ───────────────────────────────────────────────────────────────

// Re-export commonly used types from submodules
