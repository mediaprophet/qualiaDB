//! Hybrid qpr-pq-1 handshake: ML-KEM-768 then X25519 into HKDF-SHA-384.
//!
//! E02.5 splits share exchange, failure gates, and identity binding. The
//! qualified profile keeps 0-RTT disabled (`FORBIDDEN_ZERO_RTT`).

pub mod failures;
pub mod identity;
pub mod shares;

pub use failures::{
    admit_early_application_data, entropy_fill, qualified_application_data,
    qualified_handshake_gate,
};
pub use identity::{
    identity_binding, reject_reflected_share, reject_unknown_key_share, require_bound_identities,
    require_identity_binding,
};
pub use shares::{
    initiator_complete, initiator_share, responder_complete, HandshakeState, InitiatorShare,
    ResponderShare, FORBIDDEN_ZERO_RTT,
};

pub use super::schedule::derive_handshake_keys;
pub use super::schedule::HandshakeKeys;
