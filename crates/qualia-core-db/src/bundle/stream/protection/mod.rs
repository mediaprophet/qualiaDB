//! Private protected-v1 candidates; application authority and durable publication remain separate.
mod coverage;
mod error;
mod source;
pub(crate) use coverage::{EntryRange, ObjectKind, SourceObject};
pub(crate) use error::ProtectionError;
pub(crate) use source::PreparedSource;
#[cfg(test)]
mod source_tests;

mod finish;
mod geometry;
mod ids;
mod metadata;
mod publisher;
mod seal;
mod wrap;
pub(crate) use geometry::PreparedGeometry;
pub(crate) use ids::Mapping;
pub(crate) use metadata::PrivateReferences;
pub(crate) use publisher::{
    Artifact, CandidateSink, ProtectedCandidate, ProtectedPublisher, PublisherError,
};
pub(crate) use wrap::{
    authorize_and_wrap, authorize_and_wrap_remote, RecipientAuthorityProof, WrapError,
};
#[cfg(test)]
mod publisher_tests;
#[cfg(test)]
mod wrap_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod wrap_remote_tests;
