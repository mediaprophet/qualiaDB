//! Biological Entity Types
//!
//! This module defines the biological entity types and their quantum computation requirements.

use super::quantum_state::QuantumState;
use crate::NQuin;

/// Biological Entity mapped to 48-byte Super-Quin
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BiologicalEntity {
    /// 48-byte Super-Quin identifier
    pub quin: NQuin,
    /// Entity type (enzyme, protein, DNA, etc.)
    pub entity_type: BiologicalEntityType,
    /// Quantum computation type required
    pub computation_type: QuantumComputationType,
    /// Current quantum state approximation
    pub quantum_state: QuantumState,
}

/// Biological Entity Types
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Default)]
pub enum BiologicalEntityType {
    #[default]
    Protein = 1,
    Enzyme = 0,
    DNA = 2,
    RNA = 3,
    RadicalPair = 4,
    ElectronTunnel = 5,
    ProtonTunnel = 6,
    Receptor = 7,
    Ligand = 8,
}

/// Quantum Computation Types
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Default)]
pub enum QuantumComputationType {
    #[default]
    HamiltonianMapping = 6,
    ElectronTunneling = 0,
    RadicalPairMechanism = 1,
    ProtonTunneling = 2,
    DrugReceptorBinding = 3,
    EnzymeCatalysis = 4,
    WaveFunctionCollapse = 5,
}

impl Default for BiologicalEntity {
    fn default() -> Self {
        Self {
            quin: NQuin::default(),
            entity_type: BiologicalEntityType::default(),
            computation_type: QuantumComputationType::default(),
            quantum_state: QuantumState::default(),
        }
    }
}
