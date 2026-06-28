//! Quantum Biology Orchestrator
//!
//! This module provides the main orchestrator for quantum biology computations.
//! It manages the mapping of biological entities to quantum computations
//! without allocating on the heap, using only stack-based operations and fixed-size buffers.

use super::context::QuantumBiologyContext;
use super::entities::{BiologicalEntity, BiologicalEntityType, QuantumComputationType};
use super::gpu_pipeline::{GPUComputationState, QuantumGPUPipeline};
use super::qpu_bridge::QPUBridge;
use super::quantum_state::QuantumState;
use super::results::QuantumBiologyResult;
use crate::csd_storage::CsdManager;
use crate::NQuin;

/// Quantum Biology Orchestrator - Semantic Router for Biological Entities
///
/// This struct manages the mapping of biological entities to quantum computations
/// without allocating on the heap, using only stack-based operations and fixed-size buffers.
#[repr(C)]
pub struct QuantumBiologyOrchestrator {
    /// Fixed-size buffer for biological entity mappings (48-byte Super-Quins)
    entity_mappings: [BiologicalEntity; 256],
    /// GPU compute pipeline for quantum approximations
    gpu_pipeline: Option<QuantumGPUPipeline>,
    /// QPU bridge for exact Hamiltonian mapping
    qpu_bridge: Option<QPUBridge>,
    /// Current active computation count
    active_computations: u16,
}

/// Quantum Biology Error
#[derive(Debug, Clone, PartialEq)]
pub enum QuantumBiologyError {
    InvalidBufferSize,
    GPUInitializationFailed,
    InvalidCredentials,
    EntityMappingFull,
    InvalidEntityIndex,
    InvalidEntityType,
    GPUNotAvailable,
    GPUBusy,
    ComputationTimeout,
    QPUNotAvailable,
    QPUNotConnected,
    InvalidGPUBuffer,
    GPUSubmissionFailed,
    QPUSubmissionFailed,
    InputBufferOverflow,
}

impl core::fmt::Display for QuantumBiologyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            QuantumBiologyError::InvalidBufferSize => write!(f, "Invalid buffer size"),
            QuantumBiologyError::GPUInitializationFailed => write!(f, "GPU initialization failed"),
            QuantumBiologyError::InvalidCredentials => write!(f, "Invalid credentials"),
            QuantumBiologyError::EntityMappingFull => write!(f, "Entity mapping full"),
            QuantumBiologyError::InvalidEntityIndex => write!(f, "Invalid entity index"),
            QuantumBiologyError::InvalidEntityType => write!(f, "Invalid entity type"),
            QuantumBiologyError::GPUNotAvailable => write!(f, "GPU not available"),
            QuantumBiologyError::GPUBusy => write!(f, "GPU busy"),
            QuantumBiologyError::ComputationTimeout => write!(f, "Computation timeout"),
            QuantumBiologyError::QPUNotAvailable => write!(f, "QPU not available"),
            QuantumBiologyError::QPUNotConnected => write!(f, "QPU not connected"),
            QuantumBiologyError::InvalidGPUBuffer => write!(f, "Invalid GPU buffer"),
            QuantumBiologyError::GPUSubmissionFailed => write!(f, "GPU submission failed"),
            QuantumBiologyError::QPUSubmissionFailed => write!(f, "QPU submission failed"),
            QuantumBiologyError::InputBufferOverflow => write!(f, "Input buffer overflow"),
        }
    }
}

impl QuantumBiologyOrchestrator {
    /// Create new quantum biology orchestrator with zero allocation
    pub fn new() -> Self {
        Self {
            entity_mappings: [BiologicalEntity::default(); 256],
            gpu_pipeline: None,
            qpu_bridge: None,
            active_computations: 0,
        }
    }

    /// Register biological entity for quantum computation (zero allocation)
    pub fn register_entity(
        &mut self,
        quin: NQuin,
        entity_type: BiologicalEntityType,
        computation_type: QuantumComputationType,
    ) -> Result<usize, QuantumBiologyError> {
        // Find empty slot in entity mappings
        for i in 0..256 {
            if self.entity_mappings[i].quin.subject == 0
                && self.entity_mappings[i].quin.predicate == 0
            {
                self.entity_mappings[i] = BiologicalEntity {
                    quin,
                    entity_type,
                    computation_type,
                    quantum_state: QuantumState::default(),
                };
                return Ok(i);
            }
        }

        Err(QuantumBiologyError::EntityMappingFull)
    }

    /// Get current active computation count
    pub fn active_computations(&self) -> u16 {
        self.active_computations
    }
}

impl Default for QuantumBiologyOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}
