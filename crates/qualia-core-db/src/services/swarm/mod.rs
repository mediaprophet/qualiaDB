//! **The Swarm — verify-before-pay distributed jobs.**
//!
//! A swarm job is work that one node cannot (or should not) do alone, dispatched
//! across the socially-defined network in one of three [`job::JobMode`]s:
//!
//! * **Personal** — your own devices cooperate (no payment).
//! * **Collaborative** — done with named peers (no payment).
//! * **Paid** — dispatched to a provider for payment (the *solar-excess* case: a node
//!   with surplus renewable energy sells idle compute).
//!
//! ## The load-bearing invariant: verify before you pay
//!
//! A paid swarm is dual-use. The same dispatch is sovereignty (your fabric works for
//! you) *or* extraction (you pay for fabricated or wrong work, or a node lies about
//! what it computed). What decides which is a **result-verification gate that runs
//! before any payment instruction is emitted**:
//!
//! ```text
//!   spec ──► execute (untrusted provider) ──► VERIFY (trusted local reference)
//!                                                  │
//!                                       Verified ──┴── Rejected
//!                                          │             │
//!                                    emit Pay         emit Refund
//!                                    instruction      (no provider payment)
//! ```
//!
//! Verification never trusts the executor — it re-derives correctness with a cheap
//! **trusted reference** (Freivalds' algorithm for matrix products in O(n²); ranking
//! reproduction for embedding artifacts; see [`verify`]). Only a `Verified` verdict
//! lets [`settlement`] emit a [`crate::rpc::MicropaymentInstruction`]. **This library
//! never moves funds** — it emits the instruction that the existing
//! [`crate::ilp_dispatcher`] (the actual rail) executes under human authorisation.
//!
//! ## Reuse, not reinvention
//!
//! * Compute reuses [`crate::solvers::linear_algebra`] (matmul/matvec) and
//!   [`crate::solvers::learning::kg_embedding`] (the real KGE trainer).
//! * Money arithmetic reuses [`crate::modalities::value_flow`] (pool/discharge,
//!   `eroi_viable` — the thermodynamic supply gate that refuses net-extractive jobs).
//! * Payment transport reuses [`crate::ilp_dispatcher`].
//!
//! Kernel-class boundaries are explicit and CPU references are always present (§13):
//! the executor is dispatch-ready, the verifier is the always-present CPU oracle.

#![cfg(not(target_arch = "wasm32"))]

pub mod dispatch;
pub mod executor;
pub mod isolate;
pub mod job;
pub mod settlement;
pub mod verify;

pub use dispatch::{run_job, DispatchOutcome};
pub use executor::{JobExecutor, LocalKernelExecutor};
pub use isolate::isolate_b_compute;
pub use job::{content_id, JobInput, JobKind, JobMode, JobResult, JobSpec};
pub use settlement::{price_paid_job, Escrow, EscrowState, SettlementOutcome};
pub use verify::{verify, VerificationVerdict, VerifyPolicy};

/// Fail-closed errors for swarm job handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwarmError {
    /// Input dimensions are inconsistent for the declared kind.
    InvalidJob,
    /// A reused kernel (GEMM, trainer) failed.
    KernelFailed,
    /// The escrow was not in a state allowing the requested transition.
    InvalidEscrowState,
    /// A paid job's energy economics are net-extractive (E-ROI below floor) — refused.
    NotEnergyViable,
}

impl core::fmt::Display for SwarmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SwarmError::InvalidJob => write!(f, "inconsistent job input"),
            SwarmError::KernelFailed => write!(f, "reused compute kernel failed"),
            SwarmError::InvalidEscrowState => write!(f, "invalid escrow state transition"),
            SwarmError::NotEnergyViable => {
                write!(f, "paid job is net-extractive (E-ROI below floor)")
            }
        }
    }
}
impl std::error::Error for SwarmError {}
