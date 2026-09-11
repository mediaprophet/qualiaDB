//! Verified authority types. Trusted constructors stay crate-private.

mod decoded;
mod digest_bind;
mod owner;
mod permit;
mod policy;
mod verified;

pub use decoded::DecodedClaim;
pub use digest_bind::{bind_authority_digests, AuthorityBinding};
pub use owner::{binding_for_controllers, AuthorityOwner, MutationHandle};
pub use permit::ExecutionPermit;
pub use policy::{
    admit_service, bootstrap_admit, collapse_to_trusted_peer, evaluate_precedence, CachedGrant,
    CompensationClass, ContactState, IndependentFacts, ObservationQuality, Plane, PolicyOutcome,
    ResourceKind, TemporalGrant,
};
pub use verified::{AuthorisedContact, InstalledSessionKeys, VerifiedCredential};
