//! Signed V3 evidence and live native acceptance through the existing authority.
//! Source labels describe supplied evidence; only separately qualified runtime
//! adapters establish its actual origin, model backing and delivery semantics.
mod artifact_binding;
mod artifact_digest;
#[cfg(not(target_arch = "wasm32"))]
mod artifact_session;
#[cfg(not(target_arch = "wasm32"))]
mod controller;
mod encoding;
mod evaluate;
mod policy;
mod proposal;
mod requirements;
#[cfg(not(target_arch = "wasm32"))]
mod retained;
#[cfg(not(target_arch = "wasm32"))]
mod session;
#[cfg(not(target_arch = "wasm32"))]
mod token_session;
pub use artifact_binding::BoundArtifact;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use controller::EvidenceController;
pub use evaluate::evaluate_constraints;
pub use policy::{
    policy_digest_v3, EvidencePolicy, SignedEvidencePolicy, VerifiedEvidencePolicy, AUDIENCE_V3,
};
pub use proposal::{
    ArtifactMedia, ArtifactOrigin, BoundProposal, CandidateSource, NumericalObservation, Payload,
    Proposal, ScoreMeaning,
};
pub use requirements::{ObservationRequirements, ScoreRequirement};
#[cfg(not(target_arch = "wasm32"))]
pub use retained::{AcceptedEvidence, AcceptedKey, EvidenceLimits, EvidenceStorage};
#[cfg(not(target_arch = "wasm32"))]
pub use session::{GovernedEvidenceSession, PendingEvidence};
#[cfg(all(test, not(target_arch = "wasm32")))]
mod bridge_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod integration_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod schema_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_actor;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
