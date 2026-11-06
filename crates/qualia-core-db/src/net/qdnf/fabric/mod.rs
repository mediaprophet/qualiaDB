//! Identifier fabric planes (FND-02.01, IDF-02, NET-21).
//!
//! Entity, claim, handle and instrument stay distinct. Join attempts via DID,
//! wallet, location, role, alias, classifier similarity, `owl:sameAs`, biometric
//! sample, account handle, sensor co-presence, org representative, or reliance
//! retraction are refused. A verified route or instrument is not a NaturalAgent.

pub mod instruments;
pub mod join;
pub mod locator;
pub mod referent;

pub use instruments::{
    shared_generator_factor, CoAttestationBundle, EntityAgentKind, InstrumentKind,
    ProvenanceMonomial, MAX_ATTESTATION_MEMBERS, MAX_MONOMIAL_GENERATORS,
};
pub use join::{
    attempt_join, validate_evidence_relation, EvidenceRelation, JoinAttempt, JoinKey,
};
pub use locator::{validate_locator_use, AudienceScope, RelationScopedLocator};
pub use referent::{as_natural_agent, NetworkReferent};
