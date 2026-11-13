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
pub(crate) use geometry::PreparedGeometry;
pub(crate) use ids::Mapping;
pub(crate) use metadata::PrivateReferences;
pub(crate) use publisher::{
    Artifact, CandidateSink, ProtectedCandidate, ProtectedPublisher, PublisherError,
};
#[cfg(test)]
mod publisher_tests;
