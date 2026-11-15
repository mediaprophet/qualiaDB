//! Private protected-v1 candidates; application authority and durable publication remain separate.
mod coverage;
mod error;
mod source;
pub use coverage::{EntryRange, ObjectKind, SourceObject};
pub use error::ProtectionError;
pub use source::PreparedSource;
#[cfg(test)]
mod source_tests;

mod finish;
mod geometry;
mod ids;
mod metadata;
mod publisher;
mod seal;
mod wrap;
pub use geometry::{PreparedGeometry, EMPTY_REF, EMPTY_ROOT};
pub use ids::Mapping;
pub use metadata::PrivateReferences;
pub use publisher::{
    Artifact, CandidateSink, ProtectedCandidate, ProtectedPublisher, PublisherError,
};
pub use wrap::{
    authorize_and_wrap, authorize_and_wrap_remote, RecipientAuthorityProof, WrapError,
};
#[cfg(test)]
mod publisher_tests;
#[cfg(test)]
mod wrap_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod wrap_remote_tests;
