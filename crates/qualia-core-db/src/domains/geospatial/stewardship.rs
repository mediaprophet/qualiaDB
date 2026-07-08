//! Stewardship & Quorum Model
//! Implements delegated stewardship for the spatio-temporal commons.
//! Multi-steward M-of-N signatures and the explicit-denial (G2) guard.

use crate::NQuin;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct StewardshipContract {
    pub contract_id: u64,
    pub domain_bounds: (f64, f64, f64, f64), // spatial domain of authority
    pub stewards: Vec<u64>, // DIDs of the stewards
    pub quorum_m: usize,
}

#[derive(Debug, Clone)]
pub struct QuorumProposal {
    pub proposal_id: u64,
    pub contract_id: u64,
    pub signatures: HashSet<u64>,
}

impl StewardshipContract {
    /// Checks if a proposal has reached the required quorum (M-of-N).
    pub fn is_ratified(&self, proposal: &QuorumProposal) -> bool {
        let valid_signatures = proposal.signatures.iter().filter(|sig| self.stewards.contains(sig)).count();
        valid_signatures >= self.quorum_m
    }
}

/// Explicit-Denial Guard (G2 Rule).
/// Prevents automated agents from overriding human refusal.
/// If any active `DenialFact` exists in the context for an action, it cannot be overridden.
pub fn check_explicit_denial_guard(action_quin: &NQuin, context_quins: &[NQuin]) -> Result<(), &'static str> {
    // We assume metadata bit 63 denotes a denial fact
    const DENIAL_BIT: u64 = 1 << 63;
    
    for q in context_quins {
        if q.subject == action_quin.subject && (q.metadata & DENIAL_BIT) != 0 {
            return Err("Action blocked by Explicit-Denial Guard (G2 Rule)");
        }
    }
    Ok(())
}
