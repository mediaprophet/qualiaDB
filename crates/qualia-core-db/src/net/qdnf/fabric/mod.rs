//! Identifier fabric planes (FND-02.01).
//!
//! Entity, claim, handle and instrument stay distinct. Join attempts via DID,
//! wallet, location, role, alias or classifier similarity are refused. A verified
//! route or instrument is not a NaturalAgent.

pub mod join;
pub mod referent;

pub use join::{attempt_join, JoinAttempt, JoinKey};
pub use referent::{as_natural_agent, NetworkReferent};
