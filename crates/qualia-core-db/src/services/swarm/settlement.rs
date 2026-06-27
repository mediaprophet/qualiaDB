//! Escrow + settlement — the money side, gated on verification, and **incapable of
//! moving funds itself**.
//!
//! An [`Escrow`] is a small state machine over the agreed price. It is funded
//! (`Offered → Held`), then settled (`Held → ReleasedToProvider | RefundedToRequester`)
//! **only** by a verification verdict. On `Verified` it emits a
//! [`MicropaymentInstruction`] addressed to the provider; on `Rejected` it emits a
//! refund and **no** provider payment. Emitting an instruction is not the same as
//! executing it: the actual transfer is performed by the separate
//! [`crate::ilp_dispatcher`] rail, under human authorisation. Nothing here touches a
//! wallet, a connector, or the network.
//!
//! The money arithmetic (fair-price cap, energy viability) reuses
//! [`crate::modalities::value_flow`] — the single source of truth — rather than
//! re-implementing it.

use super::verify::VerificationVerdict;
use super::SwarmError;
use crate::modalities::value_flow::{commons_cost, eroi_viable};
use crate::rpc::MicropaymentInstruction;

/// Escrow lifecycle. A job is paid only by passing left-to-right through `Verified`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscrowState {
    /// Price agreed, funds not yet committed.
    Offered,
    /// Funds notionally locked pending verification (tracked here, not moved).
    Held,
    /// Verified → a payment instruction to the provider was emitted.
    ReleasedToProvider,
    /// Rejected (or failed) → the hold returns to the requester; provider unpaid.
    RefundedToRequester,
}

/// What settlement produced. A `Pay` carries an instruction for the existing ILP rail;
/// a `Refund` carries nothing payable.
#[derive(Debug, Clone, PartialEq)]
pub enum SettlementOutcome {
    /// Provider is owed payment — hand this to [`crate::ilp_dispatcher`] to execute.
    Pay(MicropaymentInstruction),
    /// No provider payment; the held amount returns to the requester.
    Refund { amount_micro_units: u64, reason: &'static str },
}

/// An escrow for one paid job.
#[derive(Debug, Clone, PartialEq)]
pub struct Escrow {
    pub job_id: u64,
    pub requester_did: u64,
    pub provider_did: u64,
    pub amount_micro_units: u64,
    /// Where the provider is paid if verified (an ILP payment pointer / address).
    pub provider_ilp: String,
    pub use_nym: bool,
    pub state: EscrowState,
}

impl Escrow {
    /// Open an escrow in the `Offered` state for an agreed price.
    pub fn offer(
        job_id: u64,
        requester_did: u64,
        provider_did: u64,
        amount_micro_units: u64,
        provider_ilp: impl Into<String>,
        use_nym: bool,
    ) -> Self {
        Self {
            job_id,
            requester_did,
            provider_did,
            amount_micro_units,
            provider_ilp: provider_ilp.into(),
            use_nym,
            state: EscrowState::Offered,
        }
    }

    /// Commit the funds to escrow (`Offered → Held`). Tracked only — no transfer.
    pub fn hold(&mut self) -> Result<(), SwarmError> {
        if self.state != EscrowState::Offered {
            return Err(SwarmError::InvalidEscrowState);
        }
        self.state = EscrowState::Held;
        Ok(())
    }

    /// Settle the escrow according to a verification verdict. Must be `Held`.
    ///
    /// * `Verified` → `ReleasedToProvider`, returns the provider payment instruction.
    /// * `Rejected` → `RefundedToRequester`, returns a refund (no provider payment).
    ///
    /// This is the only path to a provider payment, and it is impossible to reach
    /// without a `Verified` verdict.
    pub fn settle(
        &mut self,
        verdict: VerificationVerdict,
    ) -> Result<SettlementOutcome, SwarmError> {
        if self.state != EscrowState::Held {
            return Err(SwarmError::InvalidEscrowState);
        }
        match verdict {
            VerificationVerdict::Verified { .. } => {
                self.state = EscrowState::ReleasedToProvider;
                Ok(SettlementOutcome::Pay(MicropaymentInstruction {
                    recipient_label: format!("swarm-provider:{:016x}", self.provider_did),
                    ilp_address: self.provider_ilp.clone(),
                    amount_micro_cents: self.amount_micro_units,
                    use_nym: self.use_nym,
                }))
            }
            VerificationVerdict::Rejected { reason } => {
                self.state = EscrowState::RefundedToRequester;
                Ok(SettlementOutcome::Refund {
                    amount_micro_units: self.amount_micro_units,
                    reason,
                })
            }
        }
    }
}

/// Fair price for a paid job: the audited production (energy) cost plus a **capped**
/// ROI margin (the extraction guard), via [`commons_cost`]. The price never exceeds
/// `production_cost × (1 + max_roi%)`.
pub fn price_paid_job(production_cost: u64, roi_cap_percent: u64, max_roi_percent: u64) -> u64 {
    commons_cost(production_cost, roi_cap_percent, max_roi_percent)
}

/// The **solar-excess viability gate**: a paid job should only be dispatched to an
/// energy-supplier node if the value returned justifies the energy spent — E-ROI at or
/// above `min_ratio`. Below the floor the job is net-extractive and must be refused.
/// Reuses [`eroi_viable`] (the thermodynamic cost cap).
pub fn energy_viable(value_returned: u64, energy_invested: u64, min_ratio: f32) -> bool {
    eroi_viable(value_returned, energy_invested, min_ratio)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held_escrow() -> Escrow {
        let mut e = Escrow::offer(0xB0B_u64, 1, 2, 500, "$ilp.solar.node/pay", false);
        e.hold().unwrap();
        e
    }

    #[test]
    fn verified_releases_a_payment_instruction() {
        let mut e = held_escrow();
        let out = e.settle(VerificationVerdict::Verified { confidence: 0.999 }).unwrap();
        assert_eq!(e.state, EscrowState::ReleasedToProvider);
        match out {
            SettlementOutcome::Pay(instr) => {
                assert_eq!(instr.amount_micro_cents, 500);
                assert_eq!(instr.ilp_address, "$ilp.solar.node/pay");
            }
            _ => panic!("verified must produce a payment instruction"),
        }
    }

    #[test]
    fn rejected_refunds_and_pays_no_provider() {
        let mut e = held_escrow();
        let out = e.settle(VerificationVerdict::Rejected { reason: "A·B ≠ C (Freivalds)" }).unwrap();
        assert_eq!(e.state, EscrowState::RefundedToRequester);
        assert!(matches!(out, SettlementOutcome::Refund { amount_micro_units: 500, .. }));
    }

    #[test]
    fn cannot_settle_before_holding() {
        let mut e = Escrow::offer(1, 1, 2, 500, "$ilp/x", false); // still Offered
        assert_eq!(
            e.settle(VerificationVerdict::Verified { confidence: 1.0 }).unwrap_err(),
            SwarmError::InvalidEscrowState
        );
    }

    #[test]
    fn cannot_double_settle() {
        let mut e = held_escrow();
        e.settle(VerificationVerdict::Verified { confidence: 1.0 }).unwrap();
        // A second settle is now an invalid transition (already released).
        assert!(e.settle(VerificationVerdict::Verified { confidence: 1.0 }).is_err());
    }

    #[test]
    fn price_is_roi_capped() {
        // 1000 energy cost, asked 50% ROI, cap 20% → 1200.
        assert_eq!(price_paid_job(1000, 50, 20), 1200);
    }

    #[test]
    fn energy_gate_refuses_net_extractive_jobs() {
        assert!(energy_viable(300, 100, 2.0)); // 3× return
        assert!(!energy_viable(150, 100, 2.0)); // 1.5× < floor → refuse
    }
}
