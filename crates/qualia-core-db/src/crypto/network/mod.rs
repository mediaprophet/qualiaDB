//! Network-facing crypto adapters over existing Qualia primitives.

pub mod aead;
pub mod authority;
pub mod chunks;
pub mod cose;
// The raw-message verification profile is portable; custody remains native.
#[path = "../../specialized_libs/cryptographic_library/signing_profile.rs"]
pub mod signing_profile;
pub mod digest;
pub mod dual_sign;
pub mod ed25519;
pub mod entropy;
pub mod errors;
pub mod hardware;
pub mod kdf;
pub mod kem;
pub mod key_provider;
pub mod malformed;
pub mod minima;
pub mod mldsa;
pub mod pq_handshake;
pub mod rotation;
pub mod secret_lease;
pub mod share_encoding;
pub mod transcript;
pub mod types;
#[cfg(test)]
pub mod vectors;
pub mod x25519;

pub use entropy::{fill_os, EphemeralKeyLease};
pub use errors::CryptoError;
pub use key_provider::{HardwareBackend, ProviderGrant, SoftwareBackend};
pub use rotation::{RecoveredPossession, RotationState};
pub use types::{AlgorithmSpec, KeyEpoch, KeyPurpose};

pub mod object_stream;
