//! Lab experiment receipt types and operator metrics (W3: EOS-040).
//!
//! Enforces:
//! - Strict separation of host-to-device transfer bandwidth vs device-local memory bandwidth (§7).
//! - Clear division of cold setup (LUT preparation) vs warm decode latencies.
//! - Accurate tracking of stored, mapped, resident, and scratch memory domains.
//! - Explicit fidelity contract evaluation (no fake numbers or unmeasured claims).

use serde::{Deserialize, Serialize};
use crate::inference::operator_package::FidelityContract;

/// Memory footprint breakdown across separate memory domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorMemoryBreakdown {
    /// Bytes occupied in container format on persistent storage.
    pub stored_bytes: u64,
    /// Bytes mapped into virtual address space.
    pub mapped_bytes: u64,
    /// Resident physical pages held in memory.
    pub resident_bytes: u64,
    /// Caller-supplied temporary scratch space.
    pub workspace_bytes: usize,
}

impl OperatorMemoryBreakdown {
    pub fn new(stored_bytes: u64, mapped_bytes: u64, resident_bytes: u64, workspace_bytes: usize) -> Self {
        Self {
            stored_bytes,
            mapped_bytes,
            resident_bytes,
            workspace_bytes,
        }
    }
}

/// Structured lab experiment receipt recording operator conversion, execution, and fidelity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorExperimentReceipt {
    pub experiment_id: String,
    pub operator_digest: u64,
    pub source_digest: u64,
    pub fidelity_contract: FidelityContract,
    /// Host-to-device upload bandwidth in GB/s. `None` if unmeasured.
    pub transfer_bandwidth_gbps: Option<f64>,
    /// Sustained device-local memory read bandwidth in GB/s. `None` if unmeasured.
    pub local_bandwidth_gbps: Option<f64>,
    /// Time in nanoseconds spent building lookup tables or preparing execution.
    pub lut_setup_ns: u64,
    /// Latency in nanoseconds for cold first-token execution.
    pub cold_latency_ns: u64,
    /// Latency in nanoseconds for steady-state warm decode.
    pub warm_latency_ns: u64,
    /// Memory accounting across domains.
    pub memory: OperatorMemoryBreakdown,
    /// Maximum absolute numerical difference compared to scalar oracle.
    pub numerical_max_error: f32,
    /// Mean absolute numerical difference compared to scalar oracle.
    pub numerical_mean_error: f32,
    /// Whether the declared fidelity contract was satisfied.
    pub fidelity_satisfied: bool,
}

impl OperatorExperimentReceipt {
    pub fn new(
        experiment_id: impl Into<String>,
        operator_digest: u64,
        source_digest: u64,
        fidelity_contract: FidelityContract,
        memory: OperatorMemoryBreakdown,
    ) -> Self {
        Self {
            experiment_id: experiment_id.into(),
            operator_digest,
            source_digest,
            fidelity_contract,
            transfer_bandwidth_gbps: None,
            local_bandwidth_gbps: None,
            lut_setup_ns: 0,
            cold_latency_ns: 0,
            warm_latency_ns: 0,
            memory,
            numerical_max_error: 0.0,
            numerical_mean_error: 0.0,
            fidelity_satisfied: false,
        }
    }

    /// Evaluates whether the declared fidelity contract was met under the given tolerance.
    pub fn evaluate_fidelity(&mut self, tolerance: f32) {
        match self.fidelity_contract {
            FidelityContract::SourceBytePreserving => {
                // Must be bit-exact (0 error)
                self.fidelity_satisfied = self.numerical_max_error == 0.0;
            }
            FidelityContract::OperatorFidelity => {
                // Must remain within declared tolerance
                self.fidelity_satisfied = self.numerical_max_error <= tolerance;
            }
            FidelityContract::ModelQuality => {
                // Quality budget: mean error must not exceed tolerance
                self.fidelity_satisfied = self.numerical_mean_error <= tolerance;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn experiment_receipt_serialization_round_trip() {
        let mem = OperatorMemoryBreakdown::new(144, 144, 144, 4104);
        let mut receipt = OperatorExperimentReceipt::new(
            "exp-w3-q4k-lut-001",
            0x1234_5678,
            0x8765_4321,
            FidelityContract::OperatorFidelity,
            mem,
        );
        receipt.transfer_bandwidth_gbps = Some(24.5);
        receipt.local_bandwidth_gbps = Some(450.0);
        receipt.lut_setup_ns = 1200;
        receipt.cold_latency_ns = 3500;
        receipt.warm_latency_ns = 850;
        receipt.numerical_max_error = 0.00005;
        receipt.numerical_mean_error = 0.00001;
        receipt.evaluate_fidelity(0.0001);

        assert!(receipt.fidelity_satisfied);

        let json = serde_json::to_string(&receipt).expect("serialize json");
        let deserialized: OperatorExperimentReceipt =
            serde_json::from_str(&json).expect("deserialize json");

        assert_eq!(receipt, deserialized);
        assert_ne!(
            receipt.transfer_bandwidth_gbps,
            receipt.local_bandwidth_gbps,
            "transfer and local bandwidth must remain distinct"
        );
    }

    #[test]
    fn unmeasured_bandwidths_are_none() {
        let mem = OperatorMemoryBreakdown::new(144, 144, 144, 0);
        let receipt = OperatorExperimentReceipt::new(
            "exp-unmeasured",
            1,
            1,
            FidelityContract::ModelQuality,
            mem,
        );
        assert!(receipt.transfer_bandwidth_gbps.is_none());
        assert!(receipt.local_bandwidth_gbps.is_none());
    }

    #[test]
    fn source_byte_preserving_requires_zero_error() {
        let mem = OperatorMemoryBreakdown::new(144, 144, 144, 0);
        let mut receipt = OperatorExperimentReceipt::new(
            "exp-source-preserve",
            1,
            1,
            FidelityContract::SourceBytePreserving,
            mem,
        );
        receipt.numerical_max_error = 0.0001;
        receipt.evaluate_fidelity(0.01);
        assert!(!receipt.fidelity_satisfied, "source byte preservation fails on non-zero error");

        receipt.numerical_max_error = 0.0;
        receipt.evaluate_fidelity(0.01);
        assert!(receipt.fidelity_satisfied);
    }
}
