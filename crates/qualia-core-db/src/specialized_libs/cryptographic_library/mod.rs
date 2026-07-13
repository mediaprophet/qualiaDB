//! Cryptographic Library - Quantum-Resistant Cryptographic Operations
//!
//! This module provides high-performance cryptographic operations leveraging Phase 2 enhancements:
//! - Fiduciary Cryptography (ML-DSA) for post-quantum digital signatures
//! - Zero-Knowledge Semantic Proofs for privacy-preserving cryptography
//! - Hardware-Sympathetic Storage (ZNS) for secure key storage
//! - Allocation Firewall (eBPF) for kernel-level cryptographic operations

use crate::fiduciary_crypto::{CryptoContext, MlDsaSignature, MlDsaSigner, MlDsaVcProof};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Library-ized into single-concern submodules (CLAUDE.md §11). This module
// re-exports the full public surface so every external path
// `crate::specialized_libs::cryptographic_library::<Item>` resolves as before.
// ---------------------------------------------------------------------------

mod types;
mod errors;
mod library;
mod key_management;
mod signing;
mod encryption;
mod hashing;
mod proofs;
mod security;

#[cfg(test)]
mod tests;

pub use types::*;
pub use errors::*;
pub use library::*;
pub use key_management::*;
pub use signing::*;
pub use encryption::*;
pub use hashing::*;
pub use proofs::*;
pub use security::*;
