//! Quantum context (q) for epistemic superposition and wavefunction collapse

use serde::{Deserialize, Serialize};

/// Quantum context state
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum QuantumState {
    /// Collapsed Ground Truth (q = 0)
    GroundTruth = 0,
    /// Parallel epistemic context (q > 0)
    ParallelContext = 1,
    /// Pending GSR resolution ("In Escrow")
    InEscrow = 2,
    /// Sandbox evaluation (e.g., isolated q=999)
    Sandbox = 3,
}

impl Default for QuantumState {
    fn default() -> Self {
        QuantumState::GroundTruth
    }
}

impl QuantumState {
    pub fn from_q_value(q: f32) -> Self {
        if q == 0.0 {
            QuantumState::GroundTruth
        } else if q >= 999.0 {
            QuantumState::Sandbox
        } else {
            QuantumState::ParallelContext
        }
    }

    pub fn to_q_value(&self) -> f32 {
        match self {
            QuantumState::GroundTruth => 0.0,
            QuantumState::ParallelContext => 1.0,
            QuantumState::InEscrow => 2.0,
            QuantumState::Sandbox => 999.0,
        }
    }
}
