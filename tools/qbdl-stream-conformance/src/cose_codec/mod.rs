//! Actual production codec/verifier slice. Managed custody is tested in core.
#[path = "../../../../crates/qualia-core-db/src/crypto/network/cose/types.rs"]
mod types;
#[path = "../../../../crates/qualia-core-db/src/crypto/network/cose/binding.rs"]
mod binding;
#[path = "../../../../crates/qualia-core-db/src/crypto/network/cose/cbor.rs"]
mod cbor;
#[path = "../../../../crates/qualia-core-db/src/crypto/network/cose/envelope.rs"]
mod envelope;
#[path = "../../../../crates/qualia-core-db/src/crypto/network/cose/verify.rs"]
mod verify;
#[path = "../../../../crates/qualia-core-db/src/crypto/network/cose/raw_signing.rs"]
mod raw_signing;
pub use types::*;
pub use binding::{decode_binding,encode_binding,PairedPublicKeys};
pub use envelope::{decode_pair,encode_pair,sig_structure_into,ParsedPair};
pub use verify::{verify_pair,VerifiedPair};
#[cfg(test)]
pub(crate) use raw_signing::{sign_pair_raw,SigningKeys};
#[cfg(test)]
#[path = "../../../../crates/qualia-core-db/src/crypto/network/cose/tests.rs"]
mod tests;
