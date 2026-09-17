//! Compiled contract admission. Unknown required semantics cannot become Allow.

pub mod compile;
pub mod evaluate;
pub mod generations;
pub mod identity;
pub mod provenance;

pub use compile::{compile_decision, verify_exact_bytes, ContractBundle};
pub use evaluate::{evaluate, evaluate_alias, EvaluationView};
pub use generations::{commit_compiled, recheck_permit, BoundGenerations, LiveGenerations};
pub use identity::{
    consume_as_person, consume_authority, infer_natural_person_from_handle,
    inverse_functional_transfers_authority, refuse_alias_merge, same_as_transfers_authority,
    ReferentKind,
};
pub use provenance::{admit_claim, certify_identity, contradicting_claims, ClaimRecord};
