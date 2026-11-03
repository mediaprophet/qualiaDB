//! Explicitly installed policy with human-centric protections, signed actor
//! associations and synchronous Sentinel decisions. Actor subjects and their
//! multiple identifiers remain distinct from policy installation authority.
//!
//! This foundation does not yet replace the decoder's publication paths. Host
//! installation is privileged: a public DID or a key supplied by a request is
//! never authority. Numerical rollback and output publication belong to the
//! caller; a rejected session must not reuse uncertain provider state.
#![forbid(unsafe_code)]

mod actor;
#[cfg(not(target_arch = "wasm32"))]
mod anchor;
#[cfg(not(target_arch = "wasm32"))]
mod associated;
mod candidate;
mod controller;
#[cfg(not(target_arch = "wasm32"))]
mod host;
mod policy;
mod program;
mod signature;

pub use actor::*;
#[cfg(not(target_arch = "wasm32"))]
pub use associated::*;
pub use candidate::*;
pub use controller::TokenDecision;
#[cfg(not(target_arch = "wasm32"))]
pub use host::*;
pub use policy::*;
pub use program::ConstraintInstruction;
pub use signature::{policy_digest, SignedPolicy, TrustedPolicyIdentity};

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod tests;
