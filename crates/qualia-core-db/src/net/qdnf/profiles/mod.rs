//! Hostile-environment protection profiles (E16.1). Named immutable control
//! sets. Negotiate only compatible profiles. Never silently lower protection
//! for cost (Recipe F).
//!
pub mod budget;
pub mod catalog;
pub mod negotiate;
pub mod requirements;

pub use budget::{
    crypto_work_units, estimate_overhead, handshake_bytes_hybrid, pad_to_next_bucket,
    record_bucket, OverheadBudget, P1_PADDING_BUCKET, P2_RECORD_BYTES, P3_CELL_BYTES,
    P3_COVER_BYTES_PER_SEC, P3_RELAY_HOPS,
};
pub use catalog::{
    catalog_entry, control_set_from_predicates, digest_control_list, ControlId, ControlPredicate,
    ControlSet, ProtectionProfile, MAX_CONTROLS,
};
pub use negotiate::{
    bind_version_digest, negotiate, negotiate_outcome, CostPreference, NegotiateOutcome,
    SelectedProfile,
};
pub use requirements::{
    min_profile_for_confidentiality, required_control_set, AssessedEnvironment, RequirementContext,
    SenderPolicy, TaskClass,
};
