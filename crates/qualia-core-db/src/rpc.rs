//! JSON-RPC + Tax Router Subsystem
//!
//! Responsibilities:
//!   1. Serialise engine telemetry into billing receipts (ILP / Lightning)
//!   2. Evaluate incoming provider terms against the ILP Threshold Shift License
//!   3. Split every accepted payment through the 12% Tax Router:
//!         12% → divided across the TaxRecipientSuite (ILP micropayments)
//!         88% → Principal's wallet
//!
//! The Tax Router is transport-agnostic — it produces a TaxDispatchPlan which
//! the ILP layer (or future Lightning/Nym bridge) executes as discrete micropayments.
//! Each recipient address is an ILP Payment Pointer ("$...") or a stablecoin
//! wallet address ("did:..."). Nym mixnet routing is opt-in per recipient (`use_nym`).

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

// ─── Tax Router ─────────────────────────────────────────────────────────────

/// The fixed statutory tax rate applied to all incoming payments.
/// 12% is split across the TaxRecipientSuite before the remainder
/// reaches the Principal's wallet.
pub const TAX_RATE_PERCENT: u64 = 12;

/// A single named recipient in the tax disbursement suite.
/// `ilp_address` is an ILP Payment Pointer ("$provider.example/account")
/// or a stablecoin address ("did:wallet:...").
/// `share_percent` must sum to 100 across all recipients in a suite.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaxRecipient {
    /// Human-readable label, e.g. "Federal Revenue Service"
    pub label: String,
    /// ILP Payment Pointer or stablecoin address
    pub ilp_address: String,
    /// Percentage share of the total tax pool (0–100, all must sum to 100)
    pub share_percent: u64,
    /// Whether this payment should be routed via Nym mixnet for privacy
    pub use_nym: bool,
}

/// A configured set of tax recipients for a jurisdiction.
/// Example: Federal 60%, State 30%, Municipal 10%.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxRecipientSuite {
    pub jurisdiction_did: String,
    pub recipients: Vec<TaxRecipient>,
}

impl TaxRecipientSuite {
    /// Validates that all share_percent values sum to exactly 100.
    pub fn validate(&self) -> Result<(), String> {
        let total: u64 = self.recipients.iter().map(|r| r.share_percent).sum();
        if total != 100 {
            Err(format!("TaxRecipientSuite shares sum to {total}, must be 100"))
        } else {
            Ok(())
        }
    }

    /// Returns a default cooperative suite using ILP payment pointers.
    /// These point at the Cooperative Commons escrow accounts.
    /// Replace with jurisdiction-specific addresses from the Tax Oracle.
    pub fn default_cooperative() -> Self {
        Self {
            jurisdiction_did: "did:gov:cooperative:commons".to_string(),
            recipients: vec![
                TaxRecipient {
                    label: "Cooperative Infrastructure Fund".to_string(),
                    ilp_address: "$ilp.qualia.coop/infrastructure".to_string(),
                    share_percent: 40,
                    use_nym: false,
                },
                TaxRecipient {
                    label: "Digital Rights Legal Defence".to_string(),
                    ilp_address: "$ilp.qualia.coop/legal-defence".to_string(),
                    share_percent: 30,
                    use_nym: false,
                },
                TaxRecipient {
                    label: "Open Source Sustainability Pool".to_string(),
                    ilp_address: "$ilp.qualia.coop/oss-sustainability".to_string(),
                    share_percent: 20,
                    use_nym: false,
                },
                TaxRecipient {
                    label: "Disaster Recovery Reserve".to_string(),
                    ilp_address: "$ilp.qualia.coop/disaster-reserve".to_string(),
                    share_percent: 10,
                    use_nym: false,
                },
            ],
        }
    }
}

/// A single resolved micropayment instruction in a dispatch plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MicropaymentInstruction {
    pub recipient_label: String,
    pub ilp_address: String,
    pub amount_micro_cents: u64,
    pub use_nym: bool,
}

/// The complete dispatch plan produced by the Tax Router.
/// The ILP layer executes each instruction as a discrete micropayment stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxDispatchPlan {
    /// Total gross amount received (µ-cents)
    pub gross_amount_micro_cents: u64,
    /// The 12% tax pool (µ-cents)
    pub tax_pool_micro_cents: u64,
    /// The 88% principal remainder (µ-cents)
    pub principal_remainder_micro_cents: u64,
    /// Individual micropayment instructions for each recipient
    pub instructions: Vec<MicropaymentInstruction>,
}

/// Routes a gross payment through the 12% Tax Router.
/// Produces a TaxDispatchPlan ready for ILP execution.
pub fn route_tax_payment(
    gross_amount_micro_cents: u64,
    suite: &TaxRecipientSuite,
) -> Result<TaxDispatchPlan, String> {
    suite.validate()?;

    let tax_pool = (gross_amount_micro_cents * TAX_RATE_PERCENT) / 100;
    let principal_remainder = gross_amount_micro_cents - tax_pool;

    let mut instructions = Vec::with_capacity(suite.recipients.len());
    let mut allocated: u64 = 0;

    for (i, recipient) in suite.recipients.iter().enumerate() {
        let amount = if i == suite.recipients.len() - 1 {
            // Last recipient gets the remainder to avoid rounding loss
            tax_pool - allocated
        } else {
            (tax_pool * recipient.share_percent) / 100
        };
        allocated += amount;

        instructions.push(MicropaymentInstruction {
            recipient_label: recipient.label.clone(),
            ilp_address: recipient.ilp_address.clone(),
            amount_micro_cents: amount,
            use_nym: recipient.use_nym,
        });
    }

    Ok(TaxDispatchPlan {
        gross_amount_micro_cents,
        tax_pool_micro_cents: tax_pool,
        principal_remainder_micro_cents: principal_remainder,
        instructions,
    })
}

// ─── Provider Terms Negotiation ──────────────────────────────────────────────

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
#[derive(Debug, Serialize, Deserialize)]
pub struct NegotiationResponse {
    pub status: NegotiationStatus,
    /// Full dispatch plan: 12% split across suite, 88% to Principal
    pub tax_dispatch_plan: Option<TaxDispatchPlan>,
    /// Convenience field: total µ-cents routed to tax suite
    pub tax_pool_micro_cents: u64,
    /// Convenience field: µ-cents retained by Principal
    pub principal_remainder_micro_cents: u64,
}

/// Evaluates a corporate provider's connection request against the user's
/// intrinsic ILP connectivity cost and Rights Ontology, then routes the
/// accepted payment through the 12% Tax Router.
pub fn negotiate_provider_terms(
    request: ProviderTermsRequest,
    base_connectivity_cost: u64,
    tax_suite: Option<TaxRecipientSuite>,
) -> NegotiationResponse {
    // 1. ILP Threshold Shift Check
    if request.proposed_ilp_offset < base_connectivity_cost {
        return NegotiationResponse {
            status: NegotiationStatus::Reject(
                "INSUFFICIENT_OFFSET: Proposed ILP does not cover intrinsic connectivity costs.".to_string()
            ),
            tax_dispatch_plan: None,
            tax_pool_micro_cents: 0,
            principal_remainder_micro_cents: 0,
        };
    }

    // 2. Fiduciary Supremacy Check
    if request.data_usage_intent == "STRIP_FIDUCIARY_METADATA" {
        return NegotiationResponse {
            status: NegotiationStatus::Reject(
                "VIOLATION: Data usage intent violates Knowledge Axioms (Fiduciary Supremacy).".to_string()
            ),
            tax_dispatch_plan: None,
            tax_pool_micro_cents: 0,
            principal_remainder_micro_cents: 0,
        };
    }

    // 3. Route through 12% Tax Router
    let suite = tax_suite.unwrap_or_else(TaxRecipientSuite::default_cooperative);
    let plan = route_tax_payment(request.proposed_ilp_offset, &suite)
        .unwrap_or_else(|_| TaxDispatchPlan {
            gross_amount_micro_cents: request.proposed_ilp_offset,
            tax_pool_micro_cents: 0,
            principal_remainder_micro_cents: request.proposed_ilp_offset,
            instructions: vec![],
        });

    NegotiationResponse {
        status: NegotiationStatus::Accept,
        tax_pool_micro_cents: plan.tax_pool_micro_cents,
        principal_remainder_micro_cents: plan.principal_remainder_micro_cents,
        tax_dispatch_plan: Some(plan),
    }
}

#[cfg(test)]
mod negotiation_tests {
    use super::*;

    fn default_req(offset: u64) -> ProviderTermsRequest {
        ProviderTermsRequest {
            provider_did: "did:git:corp123".to_string(),
            proposed_ilp_offset: offset,
            data_usage_intent: "standard_routing".to_string(),
            tax_jurisdiction_did: "did:gov:cooperative:commons".to_string(),
        }
    }

    #[test]
    fn test_negotiation_insufficient_funds() {
        let res = negotiate_provider_terms(default_req(4000), 5000, None);
        assert!(matches!(res.status, NegotiationStatus::Reject(_)));
        assert_eq!(res.tax_pool_micro_cents, 0);
    }

    #[test]
    fn test_negotiation_fiduciary_violation() {
        let mut req = default_req(6000);
        req.data_usage_intent = "STRIP_FIDUCIARY_METADATA".to_string();
        let res = negotiate_provider_terms(req, 5000, None);
        assert!(matches!(res.status, NegotiationStatus::Reject(_)));
    }

    #[test]
    fn test_12_percent_tax_split() {
        // 10000 µ-cents gross → 12% = 1200 tax pool → 8800 to Principal
        let res = negotiate_provider_terms(default_req(10_000), 5000, None);
        assert_eq!(res.status, NegotiationStatus::Accept);
        assert_eq!(res.tax_pool_micro_cents, 1_200);
        assert_eq!(res.principal_remainder_micro_cents, 8_800);

        let plan = res.tax_dispatch_plan.unwrap();
        // 4 recipients in default suite, shares sum to 100
        assert_eq!(plan.instructions.len(), 4);
        let disbursed: u64 = plan.instructions.iter().map(|i| i.amount_micro_cents).sum();
        assert_eq!(disbursed, 1_200, "All tax µ-cents must be fully disbursed");
    }

    #[test]
    fn test_custom_suite_two_recipients() {
        let suite = TaxRecipientSuite {
            jurisdiction_did: "did:gov:au:ato".to_string(),
            recipients: vec![
                TaxRecipient {
                    label: "ATO Federal".to_string(),
                    ilp_address: "$ilp.ato.gov.au/federal".to_string(),
                    share_percent: 70,
                    use_nym: false,
                },
                TaxRecipient {
                    label: "State Revenue NSW".to_string(),
                    ilp_address: "$ilp.revenue.nsw.gov.au/gst".to_string(),
                    share_percent: 30,
                    use_nym: false,
                },
            ],
        };
        assert!(suite.validate().is_ok());

        // 100_000 µ-cents → 12% = 12_000 tax pool
        // Federal: 70% of 12_000 = 8_400; State: 30% of 12_000 = 3_600
        let plan = route_tax_payment(100_000, &suite).unwrap();
        assert_eq!(plan.tax_pool_micro_cents, 12_000);
        assert_eq!(plan.principal_remainder_micro_cents, 88_000);
        assert_eq!(plan.instructions[0].amount_micro_cents, 8_400);
        assert_eq!(plan.instructions[1].amount_micro_cents, 3_600);
    }

    #[test]
    fn test_suite_validation_rejects_wrong_sum() {
        let bad_suite = TaxRecipientSuite {
            jurisdiction_did: "did:gov:test".to_string(),
            recipients: vec![
                TaxRecipient { label: "A".into(), ilp_address: "$a".into(), share_percent: 60, use_nym: false },
                TaxRecipient { label: "B".into(), ilp_address: "$b".into(), share_percent: 30, use_nym: false },
                // Missing 10% — deliberately wrong
            ],
        };
        assert!(bad_suite.validate().is_err());
    }

    #[test]
    fn test_no_rounding_loss() {
        // 7 recipients with awkward shares — last one absorbs rounding dust
        let suite = TaxRecipientSuite {
            jurisdiction_did: "did:gov:test:rounding".to_string(),
            recipients: (0..7).map(|i| TaxRecipient {
                label: format!("Recipient {i}"),
                ilp_address: format!("$ilp.test/{i}"),
                share_percent: if i < 6 { 14 } else { 16 }, // 6*14 + 16 = 100
                use_nym: false,
            }).collect(),
        };
        let plan = route_tax_payment(10_007, &suite).unwrap();
        let disbursed: u64 = plan.instructions.iter().map(|i| i.amount_micro_cents).sum();
        assert_eq!(disbursed, plan.tax_pool_micro_cents, "Zero rounding loss");
    }
}
