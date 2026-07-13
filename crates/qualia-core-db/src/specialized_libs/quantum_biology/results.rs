//! Quantum Biology Analysis Results
//!
//! This module defines the result types for quantum biology computations.

/// Quantum Result Types
#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
pub enum QuantumResultType {
    TunnelingProbability = 0,
    BindingAffinity = 1,
    CatalysisRate = 2,
    ReactionProbability = 3,
    EnergyLevel = 4,
    CoherenceTime = 5,
}

/// Quantum Biology Analysis Results (fixed-size, no allocation)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QuantumBiologyResult {
    /// Result type
    pub result_type: QuantumResultType,
    /// Primary result value (fixed-point)
    pub primary_value: i32,
    /// Secondary result value
    pub secondary_value: i32,
    /// Confidence score (0-1000)
    pub confidence: u16,
    /// Computation time in microseconds
    pub computation_time_us: u32,
    /// Error code (0 = success)
    pub error_code: u16,
}

impl Default for QuantumBiologyResult {
    fn default() -> Self {
        Self {
            result_type: QuantumResultType::EnergyLevel,
            primary_value: 0,
            secondary_value: 0,
            confidence: 0,
            computation_time_us: 0,
            error_code: 0,
        }
    }
}
