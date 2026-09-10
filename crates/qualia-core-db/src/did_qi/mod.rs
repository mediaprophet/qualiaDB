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
mod mailbox_bind;
mod method;
mod service;
mod utxo;
#[cfg(test)]
mod vectors;

pub use document::{
    canonical_digest, encode_canonical, encode_genesis, encode_jsonld_cold, encode_signed,
    encode_unsigned, genesis_digest, sha256_32, signed_git_object_id, unsigned_digest, AkaEntry,
    QiDocument, CREATED_UNIX_VECTOR, MAX_AKA, MAX_AKA_LEN, MAX_CANONICAL, MAX_SERVICES,
    MAX_SIGNED, MAX_UNSIGNED,
};
pub use document_decode::{
    decode_canonical, extract_proof_sig, ingest_unsigned_json, reject_forbidden_locators,
};
pub use git_object::{
    blob_object_id, encode_blob, GitObjectStore, GIT_STORE_CAP, MAX_BLOB, MAX_RECORD,
};
pub use id::{format_did, parse_did, DidQi, MAX_DID_TEXT};
pub use mailbox_bind::publish_qi_document;
pub use method::{create, deactivate, read, read_generation, update};
pub use service::{
    check_relay_only, CscpMailbox, Disclosure, LocatorClass, RelayHint, SERVICE_TYPE, MAX_HINTS,
};
pub use utxo::{
    admit_chain, apply_utxo_attestation, chain_admitted, encode_commitment_tx, extract_op_return,
    parse_chain_id, txid_display, verify_commitment, ChainId, Outpoint, UtxoCommitment,
    CONSTITUTION_MAINNET, CONSTITUTION_TESTNET, LIVE_KIND, OP_RETURN, QI_MAGIC, TOMBSTONE_OPCODE,
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
    UnsupportedChain,
}

impl QiError {
    /// Spec §4 ASCII token. Variants without a listed token keep a stable extra name.
    pub fn token(self) -> &'static str {
        match self {
            QiError::InvalidPrefix
            | QiError::RejectedMethod
            | QiError::MalformedId
            | QiError::Encoding
            | QiError::MalformedDocument => "invalid_did",
            QiError::NotFound => "not_found",
            QiError::BadSignature | QiError::InvalidKey | QiError::ControllerMismatch => {
                "invalid_proof"
            }
            QiError::StaleGeneration => "stale_generation",
            QiError::Deactivated => "deactivated",
            QiError::DirectLocatorForbidden => "direct_locator_forbidden",
            QiError::CommitmentMismatch => "utxo_commitment_mismatch",
            QiError::UnsupportedChain => "unsupported_chain",
            QiError::CanonicalTooLarge => "document_too_large",
            QiError::BufferTooSmall | QiError::StoreFull => "buffer_full",
            QiError::MalformedTx => "malformed_tx",
            QiError::MalformedChainId => "malformed_chain_id",
            QiError::TombstoneRequiresDeactivate => "tombstone_requires_deactivate",
        }
    }
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

    #[test]
    fn spec_error_tokens_are_stable() {
        assert_eq!(QiError::MalformedId.token(), "invalid_did");
        assert_eq!(QiError::RejectedMethod.token(), "invalid_did");
        assert_eq!(QiError::NotFound.token(), "not_found");
        assert_eq!(QiError::BadSignature.token(), "invalid_proof");
        assert_eq!(QiError::StaleGeneration.token(), "stale_generation");
        assert_eq!(QiError::Deactivated.token(), "deactivated");
        assert_eq!(
            QiError::DirectLocatorForbidden.token(),
            "direct_locator_forbidden"
        );
        assert_eq!(
            QiError::CommitmentMismatch.token(),
            "utxo_commitment_mismatch"
        );
        assert_eq!(QiError::UnsupportedChain.token(), "unsupported_chain");
        assert_eq!(QiError::CanonicalTooLarge.token(), "document_too_large");
        assert_eq!(QiError::BufferTooSmall.token(), "buffer_full");
    }
}
