//! Signed custody binding for canonical radio observation records.
//!
//! A receipt signs the exact bytes produced by `radio_evidence` and binds the
//! source/capture references, signer key, explicit receipt time, optional
//! policy-decision reference, and previous checkpoint. A valid Ed25519
//! signature proves only possession of the signing key for those bytes. Host
//! signer, policy, and chain authorities are separate required checks for a
//! `VerifiedByAuthority` result. No result identifies an address owner or
//! establishes truth, custody admissibility, or legal sufficiency.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

use super::{
    radio_evidence::{encode_radio_evidence_records, RadioEvidenceError, RadioEvidenceRecord},
    record::RecordClockStamp,
};

#[path = "radio_receipt_wire.rs"]
mod wire;

pub const RADIO_RECEIPT_VERSION: u16 = 1;
pub const RADIO_RECEIPT_ENCODED_LEN: usize = 328;
const RECEIPT_MAGIC: [u8; 4] = *b"RCP1";
const SIGN_DOMAIN: &[u8] = b"qualia:radio-custody-receipt:v1\0";
const CHAIN_DOMAIN: &[u8] = b"qualia:radio-custody-checkpoint:v1\0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptTimestamp {
    /// The signer did not declare a timestamp. Host authorities should reject
    /// this for policies that require temporal ordering or freshness.
    Unknown,
    /// Host-declared Unix seconds; trust and anti-rollback are external.
    UnixSeconds(u64),
    /// Qualified monotonic clock declaration, still supplied by the signer.
    Monotonic(RecordClockStamp),
    /// Raw Windows UTC ticks, 100 ns from 1601-01-01 UTC.
    WindowsUtc100ns(i64),
}

/// Optional decision references; their meaning and validity are checked by
/// the host policy authority, not by signature parsing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PolicyDecisionReference {
    grant_digest: [u8; 32],
    request_digest: [u8; 32],
    decision_commitment: [u8; 32],
}

impl PolicyDecisionReference {
    pub fn new(
        grant_digest: [u8; 32],
        request_digest: [u8; 32],
        decision_commitment: [u8; 32],
    ) -> Result<Self, RadioReceiptError> {
        if all_zero(&grant_digest) || all_zero(&request_digest) || all_zero(&decision_commitment) {
            return Err(RadioReceiptError::InvalidPolicyReference);
        }
        Ok(Self {
            grant_digest,
            request_digest,
            decision_commitment,
        })
    }

    pub const fn grant_digest(self) -> [u8; 32] {
        self.grant_digest
    }
    pub const fn request_digest(self) -> [u8; 32] {
        self.request_digest
    }
    pub const fn decision_commitment(self) -> [u8; 32] {
        self.decision_commitment
    }
}

/// A source/capture-bound signed checkpoint. Fields are private so callers
/// cannot mutate signed custody data after construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RadioCustodyReceipt {
    version: u16,
    record_id: u64,
    source_id: u64,
    capture_id: u64,
    record_digest: [u8; 32],
    sequence: u64,
    previous_checkpoint: Option<[u8; 32]>,
    timestamp: ReceiptTimestamp,
    policy: Option<PolicyDecisionReference>,
    signer_public_key: [u8; 32],
    signature: [u8; 64],
}

impl RadioCustodyReceipt {
    /// Encode the canonical single-record bytes into caller scratch, hash those
    /// exact bytes, then sign all receipt context. Scratch is caller-owned and
    /// no allocation or file/network operation occurs.
    #[allow(clippy::too_many_arguments)]
    pub fn sign(
        record: RadioEvidenceRecord,
        record_scratch: &mut [u8],
        sequence: u64,
        previous_checkpoint: Option<[u8; 32]>,
        timestamp: ReceiptTimestamp,
        policy: Option<PolicyDecisionReference>,
        signer: &SigningKey,
    ) -> Result<Self, RadioReceiptError> {
        validate_chain_position(sequence, previous_checkpoint)?;
        if previous_checkpoint.is_some_and(|digest| all_zero(&digest)) {
            return Err(RadioReceiptError::InvalidPreviousCheckpoint);
        }
        let encoded_len = encode_radio_evidence_records(&[record], record_scratch)
            .map_err(RadioReceiptError::Record)?;
        if encoded_len != super::radio_evidence::RADIO_EVIDENCE_RECORD_SIZE {
            return Err(RadioReceiptError::InvalidRecordEncoding);
        }
        let record_digest: [u8; 32] = Sha256::digest(&record_scratch[..encoded_len]).into();
        let mut receipt = Self {
            version: RADIO_RECEIPT_VERSION,
            record_id: record.record_id(),
            source_id: record.capture().source_id(),
            capture_id: record.capture().capture_id(),
            record_digest,
            sequence,
            previous_checkpoint,
            timestamp,
            policy,
            signer_public_key: signer.verifying_key().to_bytes(),
            signature: [0; 64],
        };
        receipt.signature = signer.sign(&receipt.signing_digest()).to_bytes();
        Ok(receipt)
    }

    pub const fn version(self) -> u16 {
        self.version
    }
    pub const fn record_id(self) -> u64 {
        self.record_id
    }
    pub const fn source_id(self) -> u64 {
        self.source_id
    }
    pub const fn capture_id(self) -> u64 {
        self.capture_id
    }
    pub const fn record_digest(self) -> [u8; 32] {
        self.record_digest
    }
    pub const fn sequence(self) -> u64 {
        self.sequence
    }
    pub const fn previous_checkpoint(self) -> Option<[u8; 32]> {
        self.previous_checkpoint
    }
    pub const fn timestamp(self) -> ReceiptTimestamp {
        self.timestamp
    }
    pub const fn policy(self) -> Option<PolicyDecisionReference> {
        self.policy
    }
    pub const fn signer_public_key(self) -> [u8; 32] {
        self.signer_public_key
    }
    pub const fn signature(self) -> [u8; 64] {
        self.signature
    }

    /// Cryptographic signature and exact-record check only. The returned
    /// `SignatureValidUntrusted` status deliberately does not imply signer
    /// authorization, permission validity, chain completeness, or truth.
    pub fn verify_signature(
        &self,
        record: RadioEvidenceRecord,
        record_scratch: &mut [u8],
    ) -> Result<ReceiptVerificationStatus, RadioReceiptError> {
        self.validate_record_bytes(record, record_scratch)?;
        let key = VerifyingKey::from_bytes(&self.signer_public_key)
            .map_err(|_| RadioReceiptError::InvalidSignerKey)?;
        let signature = Signature::from_bytes(&self.signature);
        key.verify_strict(&self.signing_digest(), &signature)
            .map_err(|_| RadioReceiptError::InvalidSignature)?;
        Ok(ReceiptVerificationStatus::SignatureValidUntrusted)
    }

    /// Recheck signer authorization, optional policy decision, and chain
    /// position through host authorities after verifying signature and exact
    /// record bytes. Authority implementations must use current trust,
    /// revocation, and checkpoint-completeness state; no default is trusted.
    pub fn verify_with_authority(
        &self,
        record: RadioEvidenceRecord,
        record_scratch: &mut [u8],
        authority: &impl RadioReceiptAuthority,
    ) -> Result<ReceiptVerificationStatus, RadioReceiptError> {
        self.verify_signature(record, record_scratch)?;
        if self.version != RADIO_RECEIPT_VERSION
            || !authority.signer_authorized(self.source_id, &self.signer_public_key)
        {
            return Err(RadioReceiptError::SignerNotTrusted);
        }
        if !authority.permission_context_valid(&record) {
            return Err(RadioReceiptError::PermissionContextNotTrusted);
        }
        if let Some(policy) = self.policy {
            if record
                .permission()
                .is_some_and(|context| context.grant_digest() != policy.grant_digest())
            {
                return Err(RadioReceiptError::PolicyBindingMismatch);
            }
            if !authority.policy_decision_valid(&self, &record, policy) {
                return Err(RadioReceiptError::PolicyDecisionNotTrusted);
            }
        }
        let checkpoint = self.checkpoint_digest();
        if !authority.chain_position_valid(
            self.sequence,
            self.previous_checkpoint.as_ref(),
            self.timestamp,
            &checkpoint,
        ) {
            return Err(RadioReceiptError::ChainNotTrusted);
        }
        Ok(ReceiptVerificationStatus::VerifiedByAuthority)
    }

    /// Canonical digest that the Ed25519 key signs.
    pub fn signing_digest(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(SIGN_DOMAIN);
        hash.update(self.version.to_le_bytes());
        hash.update(self.record_id.to_le_bytes());
        hash.update(self.source_id.to_le_bytes());
        hash.update(self.capture_id.to_le_bytes());
        hash.update(self.record_digest);
        hash.update(self.sequence.to_le_bytes());
        hash_previous(&mut hash, self.previous_checkpoint);
        hash_timestamp(&mut hash, self.timestamp);
        hash_policy(&mut hash, self.policy);
        hash.update(self.signer_public_key);
        hash.finalize().into()
    }

    /// Chain link digest includes both canonical signed content and signature.
    pub fn checkpoint_digest(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(CHAIN_DOMAIN);
        hash.update(self.signing_digest());
        hash.update(self.signature);
        hash.finalize().into()
    }

    /// Encode the signed fixed-size transport value into caller-owned output.
    /// Parsing or storage does not establish trust.
    pub fn encode_into(&self, out: &mut [u8]) -> Result<usize, RadioReceiptError> {
        wire::encode(self, out)
    }

    /// Parse the fixed v1 transport bytes. Parsing is structural only; call
    /// `verify_with_authority` before trusting the signer, policy, timestamp,
    /// or checkpoint.
    pub fn decode_from(bytes: &[u8]) -> Result<Self, RadioReceiptError> {
        wire::decode(bytes)
    }

    fn validate_record_bytes(
        &self,
        record: RadioEvidenceRecord,
        record_scratch: &mut [u8],
    ) -> Result<(), RadioReceiptError> {
        if self.version != RADIO_RECEIPT_VERSION
            || self.record_id != record.record_id()
            || self.source_id != record.capture().source_id()
            || self.capture_id != record.capture().capture_id()
        {
            return Err(RadioReceiptError::RecordMismatch);
        }
        let encoded_len = encode_radio_evidence_records(&[record], record_scratch)
            .map_err(RadioReceiptError::Record)?;
        if encoded_len != super::radio_evidence::RADIO_EVIDENCE_RECORD_SIZE {
            return Err(RadioReceiptError::InvalidRecordEncoding);
        }
        let digest: [u8; 32] = Sha256::digest(&record_scratch[..encoded_len]).into();
        if digest != self.record_digest {
            return Err(RadioReceiptError::RecordMismatch);
        }
        Ok(())
    }
}

/// A cryptographically valid signature is still unverified until host trust
/// and chain authorities accept the signer, policy reference, and checkpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptVerificationStatus {
    SignatureValidUntrusted,
    VerifiedByAuthority,
}

/// Independent host trust boundary. Each method must evaluate current state;
/// this trait deliberately has no permissive default implementation. The
/// chain authority receives the signed timestamp verbatim and must explicitly
/// accept or reject `ReceiptTimestamp::Unknown` for its policy.
pub trait RadioReceiptAuthority {
    fn signer_authorized(&self, source_id: u64, public_key: &[u8; 32]) -> bool;

    /// Validate the optional permission context embedded in the canonical
    /// record, including live scope/revocation state. This is called even when
    /// the receipt has no separate `PolicyDecisionReference`.
    fn permission_context_valid(&self, record: &RadioEvidenceRecord) -> bool;

    fn policy_decision_valid(
        &self,
        receipt: &RadioCustodyReceipt,
        record: &RadioEvidenceRecord,
        policy: PolicyDecisionReference,
    ) -> bool;

    fn chain_position_valid(
        &self,
        sequence: u64,
        previous_checkpoint: Option<&[u8; 32]>,
        timestamp: ReceiptTimestamp,
        current_checkpoint: &[u8; 32],
    ) -> bool;
}

fn validate_chain_position(
    sequence: u64,
    previous: Option<[u8; 32]>,
) -> Result<(), RadioReceiptError> {
    if (sequence == 0) != previous.is_none() {
        return Err(RadioReceiptError::InvalidChainPosition);
    }
    Ok(())
}

fn hash_previous(hash: &mut Sha256, previous: Option<[u8; 32]>) {
    match previous {
        Some(digest) => {
            hash.update([1]);
            hash.update(digest);
        }
        None => {
            hash.update([0]);
            hash.update([0; 32]);
        }
    }
}

fn hash_timestamp(hash: &mut Sha256, timestamp: ReceiptTimestamp) {
    match timestamp {
        ReceiptTimestamp::UnixSeconds(seconds) => {
            hash.update([1]);
            hash.update(seconds.to_le_bytes());
        }
        ReceiptTimestamp::Monotonic(stamp) => {
            hash.update([2]);
            hash.update(stamp.clock_id().to_le_bytes());
            hash.update(stamp.restart_epoch().to_le_bytes());
            hash.update(stamp.ticks_ns().to_le_bytes());
            hash.update(stamp.uncertainty_ns().to_le_bytes());
        }
        ReceiptTimestamp::WindowsUtc100ns(ticks) => {
            hash.update([3]);
            hash.update(ticks.to_le_bytes());
        }
        ReceiptTimestamp::Unknown => {
            hash.update([0]);
            hash.update([0; 28]);
        }
    }
}

fn hash_policy(hash: &mut Sha256, policy: Option<PolicyDecisionReference>) {
    match policy {
        Some(policy) => {
            hash.update([1]);
            hash.update(policy.grant_digest);
            hash.update(policy.request_digest);
            hash.update(policy.decision_commitment);
        }
        None => {
            hash.update([0]);
            hash.update([0; 96]);
        }
    }
}

fn encode_previous(out: &mut [u8], cursor: &mut usize, previous: Option<[u8; 32]>) {
    match previous {
        Some(digest) => {
            put(out, cursor, &[1]);
            put(out, cursor, &digest);
        }
        None => {
            put(out, cursor, &[0]);
            put(out, cursor, &[0; 32]);
        }
    }
}

fn encode_timestamp(out: &mut [u8], cursor: &mut usize, timestamp: ReceiptTimestamp) {
    match timestamp {
        ReceiptTimestamp::Unknown => {
            put(out, cursor, &[0]);
            put(out, cursor, &[0; 28]);
        }
        ReceiptTimestamp::UnixSeconds(seconds) => {
            put(out, cursor, &[1]);
            put(out, cursor, &seconds.to_le_bytes());
            put(out, cursor, &[0; 20]);
        }
        ReceiptTimestamp::Monotonic(stamp) => {
            put(out, cursor, &[2]);
            put(out, cursor, &stamp.clock_id().to_le_bytes());
            put(out, cursor, &stamp.restart_epoch().to_le_bytes());
            put(out, cursor, &stamp.ticks_ns().to_le_bytes());
            put(out, cursor, &stamp.uncertainty_ns().to_le_bytes());
        }
        ReceiptTimestamp::WindowsUtc100ns(ticks) => {
            put(out, cursor, &[3]);
            put(out, cursor, &ticks.to_le_bytes());
            put(out, cursor, &[0; 20]);
        }
    }
}

fn encode_policy(out: &mut [u8], cursor: &mut usize, policy: Option<PolicyDecisionReference>) {
    match policy {
        Some(policy) => {
            put(out, cursor, &[1]);
            put(out, cursor, &policy.grant_digest);
            put(out, cursor, &policy.request_digest);
            put(out, cursor, &policy.decision_commitment);
        }
        None => {
            put(out, cursor, &[0]);
            put(out, cursor, &[0; 96]);
        }
    }
}

fn put(out: &mut [u8], cursor: &mut usize, value: &[u8]) {
    let end = *cursor + value.len();
    out[*cursor..end].copy_from_slice(value);
    *cursor = end;
}

fn all_zero(bytes: &[u8]) -> bool {
    bytes.iter().all(|byte| *byte == 0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadioReceiptError {
    Record(RadioEvidenceError),
    InvalidPolicyReference,
    InvalidPreviousCheckpoint,
    InvalidChainPosition,
    InvalidRecordEncoding,
    OutputTooSmall { required: usize, available: usize },
    RecordMismatch,
    InvalidSignerKey,
    InvalidSignature,
    SignerNotTrusted,
    PolicyDecisionNotTrusted,
    ChainNotTrusted,
    PermissionContextNotTrusted,
    PolicyBindingMismatch,
    InvalidTransport,
}

#[cfg(test)]
#[path = "radio_receipt_tests.rs"]
mod tests;
