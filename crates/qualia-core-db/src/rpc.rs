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

/// A request from an external corporate provider (e.g., ISP, Telemetry Aggregator)
/// proposing terms to connect to the local Qualia-DB daemon.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderTermsRequest {
    pub provider_did: String,
    pub proposed_ilp_offset: u64, // µ-cents per GB
    pub data_usage_intent: String, // N3Logic ruleset hash
    pub tax_jurisdiction_did: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum NegotiationStatus {
    Accept,
    Reject(String),
}

/// The local agent's response to the provider's terms.
/// Handles Symmetrical Link generation and Stablecoin Escrow routing.
#[derive(Debug, Serialize, Deserialize)]
pub struct NegotiationResponse {
    pub status: NegotiationStatus,
    pub stablecoin_escrow_split: u64, // The tax portion routed to a stablecoin escrow
    pub wallet_balance_split: u64,    // The remaining profit routed to the user's wallet
}

/// Evaluates a corporate provider's connection request against the user's
/// intrinsic ILP connectivity cost and Rights Ontology.
/// It automatically splits valid micropayments into a tax escrow and a personal wallet.
pub fn negotiate_provider_terms(
    request: ProviderTermsRequest, 
    base_connectivity_cost: u64
) -> NegotiationResponse {
    // 1. ILP Economic Shift Check: Does the proposed offset meet the base intrinsic cost?
    if request.proposed_ilp_offset < base_connectivity_cost {
        return NegotiationResponse {
            status: NegotiationStatus::Reject("INSUFFICIENT_OFFSET: Proposed ILP does not cover intrinsic connectivity costs.".to_string()),
            stablecoin_escrow_split: 0,
            wallet_balance_split: 0,
        };
    }

    // 2. Fiduciary Supremacy Check: Ensure the data usage intent doesn't violate Knowledge Axioms.
    if request.data_usage_intent == "STRIP_FIDUCIARY_METADATA" {
        return NegotiationResponse {
            status: NegotiationStatus::Reject("VIOLATION: Data usage intent violates Knowledge Axioms (Fiduciary Supremacy).".to_string()),
            stablecoin_escrow_split: 0,
            wallet_balance_split: 0,
        };
    }

    // 3. Tax Rule Oracle & Escrow Routing
    // In production, this pulls the `.q42` tax ontology for `request.tax_jurisdiction_did`.
    // Here we mock a 10% tax rate derived from the ontology.
    let tax_rate = 0.10;
    let tax_escrow_amount = (request.proposed_ilp_offset as f64 * tax_rate) as u64;
    let user_profit = request.proposed_ilp_offset - tax_escrow_amount;

    NegotiationResponse {
        status: NegotiationStatus::Accept,
        stablecoin_escrow_split: tax_escrow_amount,
        wallet_balance_split: user_profit,
    }
}

#[cfg(test)]
mod negotiation_tests {
    use super::*;

    #[test]
    fn test_negotiation_insufficient_funds() {
        let req = ProviderTermsRequest {
            provider_did: "did:git:corp123".to_string(),
            proposed_ilp_offset: 4000,
            data_usage_intent: "standard_routing".to_string(),
            tax_jurisdiction_did: "did:gov:us:ny".to_string(),
        };
        // User requires 5000 µ-cents
        let res = negotiate_provider_terms(req, 5000);
        assert!(matches!(res.status, NegotiationStatus::Reject(_)));
    }

    #[test]
    fn test_negotiation_fiduciary_violation() {
        let req = ProviderTermsRequest {
            provider_did: "did:git:corp123".to_string(),
            proposed_ilp_offset: 6000, // Meets cost!
            data_usage_intent: "STRIP_FIDUCIARY_METADATA".to_string(), // Violates axiom!
            tax_jurisdiction_did: "did:gov:us:ny".to_string(),
        };
        let res = negotiate_provider_terms(req, 5000);
        assert!(matches!(res.status, NegotiationStatus::Reject(_)));
    }

    #[test]
    fn test_negotiation_success_and_tax_escrow() {
        let req = ProviderTermsRequest {
            provider_did: "did:git:corp123".to_string(),
            proposed_ilp_offset: 10000,
            data_usage_intent: "standard_routing".to_string(),
            tax_jurisdiction_did: "did:gov:us:ny".to_string(),
        };
        let res = negotiate_provider_terms(req, 5000);
        
        assert_eq!(res.status, NegotiationStatus::Accept);
        // 10% of 10000 goes to stablecoin escrow = 1000
        assert_eq!(res.stablecoin_escrow_split, 1000);
        // 90% goes to wallet = 9000
        assert_eq!(res.wallet_balance_split, 9000);
    }
}
