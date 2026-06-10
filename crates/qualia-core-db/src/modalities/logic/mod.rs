//! Logic Modality
//!
//! This module contains all logic-related modalities for the QualiaDB engine.
//! It includes deontic logic, core logic evaluation, QUBO compilation,
//! N3 rule processing, SHACL constraint compilation, and OWL conversion.

// ─── Core Logic ─────────────────────────────────────────────────────────────

pub mod core;
pub use core::{WebizenVM, WebizenCompiler, WebizenOpcode};

// ─── Deontic Logic ───────────────────────────────────────────────────────────

pub mod deontic;
pub use deontic::{OP_OBLIGATE, OP_PERMIT, OP_FORBID, DEFEATER_BIT, MAX_DEFEATER_SLOTS};

// ─── QUBO Compilation ─────────────────────────────────────────────────────────

pub mod qubo;

// ─── N3 Rule Processing ────────────────────────────────────────────────────────

pub mod n3_compiler;
pub mod n3_parser;
pub use n3_compiler::{N3OutputMode, AgentIntentFrame, MAX_CONTEXT_NAMESPACE_SLOTS, MAX_INTENT_SCOPE_SLOTS};
pub use n3_parser::{N3Parser, N3Event, Rule, RuleType, Term};

// ─── SHACL Constraint Compilation ───────────────────────────────────────────────

pub mod shacl;
pub use shacl::{ShaclCompiler, ShaclConstraint, ShaclSeverity, CompiledShape};

// ─── SHACL Extensions for New Client Features ───────────────────────────────────

pub mod shacl_extensions;
pub use shacl_extensions::{
    LogConfiguration, LogLevel, LogEntry, LogRetention, LogExportFormat,
    SystemTrayConfiguration, TrayMenuItem, TrayStatusIndicator, TrayAction,
    StorageConfiguration, NetworkConfiguration, TaxRecipientConfiguration, SecurityConfiguration,
};

// ─── SHACL Extensions for Specialized Libraries ─────────────────────────────────

pub mod specialized_libs_shacl;
pub use specialized_libs_shacl::{
    // Linear Algebra
    MatrixConfiguration, MatrixOperation, EigenDecomposition,
    // Machine Learning
    ModelConfiguration, TrainingConfiguration, InferenceConfiguration,
    // Physics Simulation
    SimulationConfiguration, BoundaryConditions, MeshConfiguration,
    // Chemistry Modeling
    MoleculeConfiguration, ReactionConfiguration, QuantumCalculation,
    // Medical Computing
    MedicalDataConfiguration, ClinicalDecisionConfiguration, MedicalImagingConfiguration,
    // Financial Modeling
    FinancialModelConfiguration, RiskCalculation, TradingConfiguration,
    // Engineering Analysis
    EngineeringSimulationConfiguration, MaterialProperties, LoadConfiguration,
    // Statistical Computing
    StatisticalAnalysisConfiguration, DistributionConfiguration, SamplingConfiguration,
    // Cryptographic Library
    CryptographicConfiguration, KeyManagementConfiguration, DigitalSignatureConfiguration,
    // QPU Bridge
    QPUConfiguration, QuantumCircuitConfiguration, QuantumAnnealingConfiguration,
    // Quantum Biology
    BiomolecularConfiguration, QuantumBiologyCalculation,
};

// ─── SHACL Extensions for Core Modalities ───────────────────────────────────────

pub mod core_modalities_shacl;
pub use core_modalities_shacl::{
    // Epistemic Logic
    EpistemicConfiguration, EpistemicQuery,
    // Paraconsistent Logic
    ParaconsistentConfiguration, ContradictionHandling,
    // Temporal LTL
    LTLConfiguration, TemporalTrace,
    // Spatio-Temporal
    SpatioTemporalConfiguration, AllenIntervalConfiguration, SpatialRegionConfiguration,
    // Graph Theory
    GraphConfiguration, GraphAnalysisConfiguration, GraphAlgorithmConfiguration,
    // Calculus
    CalculusConfiguration, ODEConfiguration, TensorProvenanceConfiguration,
    // Argumentation
    ArgumentationConfiguration, ArgumentEvaluationConfiguration,
    // Dialectical Logic
    DialecticalConfiguration, SynthesisConfiguration,
    // ASP
    ASPConfiguration, StableModelConfiguration,
    // Probabilistic
    ProbabilisticConfiguration, BayesianInferenceConfiguration,
    // DL
    DLConfiguration, DLQueryConfiguration,
    // Diffusion
    DiffusionConfiguration, DiffusionGridConfiguration,
    // Linear Logic
    LinearLogicConfiguration,
    // Control Feedback
    ControlFeedbackConfiguration, FeedbackGainConfiguration,
    // Interval Reasoning
    IntervalArithmeticConfiguration,
};

// ─── SHACL Extensions for Infrastructure ───────────────────────────────────────

pub mod infrastructure_shacl;
pub use infrastructure_shacl::{
    // Domain-Specific
    BiologicalDomainConfiguration, ChemicalDomainConfiguration, PhysicalDomainConfiguration,
    FinancialDomainConfiguration, MathematicalDomainConfiguration, GeospatialDomainConfiguration,
    // Obfuscation
    ObfuscationConfiguration, PolynomialObfuscationConfiguration, SemanticStripperConfiguration,
    DomainTransformerConfiguration, HybridStateConfiguration,
    // Solvers
    SolverConfiguration, CalculusSolverConfiguration, LinearAlgebraSolverConfiguration,
    OptimizationSolverConfiguration, QuantumOptimizerConfiguration, SymbolicLogicSolverConfiguration,
    // Geometric Algebra
    GeometricAlgebraConfiguration,
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
pub use shacl::ProteinScoringMatrix;
pub use shacl::ClinicalRiskModel;
pub use shacl::CalcComputeTarget;
