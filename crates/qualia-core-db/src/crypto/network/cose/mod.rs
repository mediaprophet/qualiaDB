//! Bounded paired COSE_Sign1 candidate for QBDL publication proofs.
//!
//! This strict profile is NOT a generic COSE implementation. Both signatures
//! authenticate one exact expected binding. A verified pair is NOT authority:
//! QBS-04 must independently admit the supplied keys and current controller.
//! Legacy contextual adapters and their qualification flags are unchanged.

mod binding;
mod cbor;
mod envelope;
#[cfg(not(target_arch = "wasm32"))]
mod managed_signing;
// Portable authority fixtures need raw signing; production custody stays native.
#[cfg(any(test, not(target_arch = "wasm32")))]
mod raw_signing;
mod types;
mod verify;

pub use binding::{decode_binding, encode_binding, PairedPublicKeys};
pub use envelope::{decode_pair, encode_pair, sig_structure_into, ParsedPair};
#[cfg(not(target_arch = "wasm32"))]
pub use managed_signing::sign_pair;
#[cfg(test)]
pub(crate) use raw_signing::{sign_pair_raw, SigningKeys};
pub use types::*;
pub use verify::{verify_pair, VerifiedPair};

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
