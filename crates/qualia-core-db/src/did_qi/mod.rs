//! Qualia Identifier (`did:qi`) runtime for ☉ Human-Centric Internet instruments.
//!
//! This is a DID-Core shaped method runtime: create / read / update / deactivate.
//! It is **not** `did:q42` (Q42 Resource Coordinate), **not** `did:hcai`, and **not**
//! Gate B. Git objects are in-process; UTXO attestation verifies caller-supplied
//! Bitcoin-family transaction bytes. There is no network, Chronik, DHT, or git daemon.
//!
//! Canonical document bytes use QCDE-1 (RFC 8785 profile) in [`document`]. The DID
//! method-specific-id is multibase Bitcoin base58btc (`z` prefix) of the 32-byte
//! SHA-256 genesis payload digest.

mod document;
mod document_decode;
mod git_object;
mod id;
mod method;
mod service;
mod utxo;
#[cfg(test)]
mod vectors;

pub use document::{
    canonical_digest, encode_canonical, encode_genesis, encode_jsonld_cold, encode_signed,
    encode_unsigned, genesis_digest, sha256_32, signed_git_object_id, unsigned_digest, AkaEntry,
    QiDocument, CREATED_UNIX_VECTOR, MAX_AKA, MAX_AKA_LEN, MAX_CANONICAL, MAX_SERVICES,
};
pub use document_decode::{decode_canonical, ingest_unsigned_json, reject_forbidden_locators};
pub use git_object::{
    blob_object_id, encode_blob, GitObjectStore, GIT_STORE_CAP, MAX_BLOB, MAX_RECORD,
};
pub use id::{format_did, parse_did, DidQi, MAX_DID_TEXT};
pub use method::{create, deactivate, read, read_generation, update};
pub use service::{
    check_relay_only, CscpMailbox, Disclosure, LocatorClass, RelayHint, SERVICE_TYPE, MAX_HINTS,
};
pub use utxo::{
    apply_utxo_attestation, encode_commitment_tx, extract_op_return, parse_chain_id,
    verify_commitment, ChainId, Outpoint, UtxoCommitment, LIVE_KIND, OP_RETURN, QI_MAGIC,
    TOMBSTONE_OPCODE,
};

/// DID method name. Bound in prose to the Human-Centric Internet (HCInet).
pub const METHOD_NAME: &str = "qi";

/// Fail-closed errors. Hot paths return these without allocating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QiError {
    InvalidPrefix,
    RejectedMethod,
    MalformedId,
    BufferTooSmall,
    Encoding,
    NotFound,
    StaleGeneration,
    Deactivated,
    BadSignature,
    InvalidKey,
    DirectLocatorForbidden,
    StoreFull,
    CanonicalTooLarge,
    CommitmentMismatch,
    MalformedTx,
    MalformedDocument,
    ControllerMismatch,
    MalformedChainId,
    TombstoneRequiresDeactivate,
}

/// In-process document store. Implementations keep a fixed slot cap.
pub trait QiStore {
    fn put(&mut self, id: &DidQi, generation: u64, payload: &[u8]) -> Result<(), QiError>;
    fn get(&self, id: &DidQi, out: &mut [u8]) -> Result<(u64, usize), QiError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_name_is_qi_not_q42_or_hcai() {
        assert_eq!(METHOD_NAME, "qi");
        assert_ne!(METHOD_NAME, "q42");
        assert_ne!(METHOD_NAME, "hcai");
        assert_ne!(METHOD_NAME, "hci");
    }
}
