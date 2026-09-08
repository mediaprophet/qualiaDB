//! Network-facing crypto adapters over existing Qualia primitives.

pub mod aead;
pub mod digest;
pub mod ed25519;
pub mod entropy;
pub mod errors;
pub mod hardware;
pub mod kdf;
pub mod kem;
pub mod key_provider;
pub mod mldsa;
pub mod pq_handshake;
pub mod rotation;
pub mod secret_lease;
pub mod transcript;
pub mod types;
pub mod x25519;

pub use entropy::{fill_os, EphemeralKeyLease};
pub use errors::CryptoError;
pub use key_provider::{HardwareBackend, ProviderGrant, SoftwareBackend};
pub use rotation::{RecoveredPossession, RotationState};
pub use types::{AlgorithmSpec, KeyEpoch, KeyPurpose};
