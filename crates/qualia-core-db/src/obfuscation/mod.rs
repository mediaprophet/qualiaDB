//! Data Obfuscation Module - Zero-Allocation Semantic Stripping
//! 
//! This module provides semantic data transformation capabilities that strip
//! human-centric context while preserving mathematical structure for QPU offloading.
//! All operations maintain strict zero-allocation invariants and map directly
//! to the 48-byte NQuin payload structure.

#![no_std]

pub mod polynomial_obfuscator;
pub mod semantic_stripper;
pub mod domain_transformer;
pub mod hybrid_state_manager;

pub use polynomial_obfuscator::PolynomialObfuscator;
pub use semantic_stripper::SemanticStripper;
pub use domain_transformer::{ObfuscationDomain, DomainTransformer};
pub use hybrid_state_manager::{HybridStateManager, HybridStateDomain, ConvergenceState};

use crate::execution_error::ExecutionError;

/// Result type for obfuscation operations
pub type ObfuscationResult<T> = Result<T, ExecutionError>;

/// Classical state for hybrid quantum-classical workflows
#[repr(C)]
pub struct ClassicalState {
    /// Current clinical inference state
    pub clinical_state: ClinicalInferenceState,
    /// Defeasible rule evaluation context
    pub defeasible_context: DefeasibleContext,
    /// Temporal pharmacokinetic state
    pub temporal_state: TemporalPharmacokineticState,
    /// Domain-specific state data
    pub domain_state: [u8; 32],
}

/// Clinical inference state preservation
#[repr(C)]
pub struct ClinicalInferenceState {
    /// Current rule evaluation state
    pub evaluation_state: u8,
    /// Active rule count
    pub active_rule_count: u8,
    /// Contradiction count
    pub contradiction_count: u8,
    /// Inference confidence
    pub inference_confidence: u16,
    /// Target rule ID for quantum enhancement
    pub target_rule_id: u64,
    /// Reserved space for future expansion
    pub reserved: [u8; 1],
}

/// Defeasible context preservation
#[repr(C)]
pub struct DefeasibleContext {
    /// Rule priority state
    pub rule_priorities: [u8; 16],
    /// Contradiction resolution state
    pub resolution_state: u8,
    /// Evaluation depth
    pub evaluation_depth: u8,
    /// Context metadata
    pub context_metadata: [u8; 14],
}

/// Temporal pharmacokinetic state
#[repr(C)]
pub struct TemporalPharmacokineticState {
    /// Current medication efficacy
    pub current_efficacy: f32,
    /// Decay factor
    pub decay_factor: f32,
    /// Time since last dose
    pub time_since_dose: u64,
    /// Half-life tracking
    pub half_life_us: u64,
}

impl Default for ClassicalState {
    fn default() -> Self {
        Self {
            clinical_state: ClinicalInferenceState::default(),
            defeasible_context: DefeasibleContext::default(),
            temporal_state: TemporalPharmacokineticState::default(),
            domain_state: [0; 32],
        }
    }
}

impl Default for ClinicalInferenceState {
    fn default() -> Self {
        Self {
            evaluation_state: 0,
            active_rule_count: 0,
            contradiction_count: 0,
            inference_confidence: 0,
            target_rule_id: 0,
            reserved: [0; 1],
        }
    }
}

impl Default for DefeasibleContext {
    fn default() -> Self {
        Self {
            rule_priorities: [0; 16],
            resolution_state: 0,
            evaluation_depth: 0,
            context_metadata: [0; 14],
        }
    }
}

impl Default for TemporalPharmacokineticState {
    fn default() -> Self {
        Self {
            current_efficacy: 0.0,
            decay_factor: 0.0,
            time_since_dose: 0,
            half_life_us: 0,
        }
    }
}
