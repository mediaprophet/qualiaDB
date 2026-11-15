//! Project authoring policy over host-authenticated, current cooperative state.
//!
//! This cold, bounded library is not an authentication provider or a second
//! membership database. The native owner supplies a borrowed view while holding
//! its admission/state lock through the authorized operation and record commit.
//! Never construct that view from Vibe/MCP arguments, a checkpoint, local roster,
//! `Identity.current_user`, or a claimed DID. Serialized records are evidence,
//! never reusable authority. Supported slice: directly authenticated members;
//! agency delegation requires a separately verified host adapter.

pub mod delegation;
mod human_authority;
pub use human_authority::{HumanAuthorityEvidence, HumanAuthorityStatus};
pub mod package_review;
mod policy;
mod records;
mod types;

pub use policy::{authorize, AuthoringPermit};
pub use records::{
    ContributionRecord, IdentifierAssociationEvidence, ReviewDecision, ReviewRecord, WorkStatus,
};
pub use types::*;

#[cfg(test)]
mod tests;
