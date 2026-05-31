//! JSON-RPC Subsystem
//! Responsible for serializing engine telemetry into standardized payloads 
//! for the Bitcoin Lightning Node proxy to settle queries on the Permissive Commons.

use serde::{Deserialize, Serialize};
use crate::telemetry::get_telemetry_snapshot;

/// A receipt detailing the exact Virtual Compute Cycles burned during a query.
#[derive(Debug, Serialize, Deserialize)]
pub struct ComputeCostReceipt {
    pub query_id: String,
    pub superblock_cost: usize,
    pub sieve_ops_cost: usize,
    pub vm_cycles_cost: usize,
    pub total_sats_owed: u64,
}

impl ComputeCostReceipt {
    /// Generates a final billing receipt based on the current telemetry snapshot.
    /// 
    /// Cost Weights (Mock values for Permissive Commons):
    /// 1 SuperBlock IO = 10 micro-sats
    /// 1 Sieve Op = 1 micro-sat
    /// 1 VM Cycle = 5 micro-sats
    pub fn generate(query_id: &str) -> Self {
        let (io, sieve, vm) = get_telemetry_snapshot();
        
        let io_cost = io * 10;
        let sieve_cost = sieve * 1;
        let vm_cost = vm * 5;
        
        let total_micro_sats = io_cost + sieve_cost + vm_cost;
        let total_sats_owed = (total_micro_sats as f64 / 1_000_000.0).ceil() as u64;

        Self {
            query_id: query_id.to_string(),
            superblock_cost: io,
            sieve_ops_cost: sieve,
            vm_cycles_cost: vm,
            total_sats_owed,
        }
    }
    
    /// Serializes the receipt to a JSON string for the external Lightning API.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{reset_telemetry, SUPERBLOCK_IO_COUNT, SIEVE_OPS_COUNT, VM_CYCLES_COUNT};
    use std::sync::atomic::Ordering;

    #[test]
    fn test_rpc_receipt_generation() {
        reset_telemetry();
        
        // Burn virtual compute cycles
        SUPERBLOCK_IO_COUNT.fetch_add(1500, Ordering::Relaxed);
        SIEVE_OPS_COUNT.fetch_add(0, Ordering::Relaxed);
        VM_CYCLES_COUNT.fetch_add(45000, Ordering::Relaxed);
        
        let receipt = ComputeCostReceipt::generate("test-tx-123");
        let json = receipt.to_json();
        
        assert!(json.contains("test-tx-123"), "JSON missing query ID");
        assert_eq!(receipt.superblock_cost, 1500);
        assert_eq!(receipt.vm_cycles_cost, 45000);
        
        // 1500 * 10 = 15,000
        // 45000 * 5 = 225,000
        // Total Micro Sats = 240,000 => 1 Sat (Ceil)
        assert_eq!(receipt.total_sats_owed, 1);
    }
}
