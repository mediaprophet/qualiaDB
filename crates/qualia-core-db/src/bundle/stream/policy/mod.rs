//! Bounded signed closure and explicit local whole-source adoption.
//! Remote Share/consent/Sentinel and consumer admission are separate open gates.
mod authenticated;
mod grant;
mod manifest;
mod types;
pub use authenticated::{AuthenticatedClosure, BoundClosure, authenticate_closure};
pub use grant::{LocalEffectControl, LocalGrantRequest, LocalWholeSourceGrant, adopt_local_whole_source};
pub use manifest::{ManifestView, ManifestArtifact, ManifestDependency, decode_manifest, encode_manifest_into};
pub use types::{
    CLOSURE_PURPOSE, MAX_MANIFEST_BYTES, ManifestFields, PUBLICATION_PURPOSE, PolicyError,
    PublicationProof,
};
#[cfg(test)]
pub(crate) mod tests;
