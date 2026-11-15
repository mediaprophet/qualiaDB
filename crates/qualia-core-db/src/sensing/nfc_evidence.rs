//! Bounded variable-length radio identifiers and signed custody receipts.
//!
//! NFC UIDs and other protocol identifiers are observation bytes only. This
//! module makes no stable-device, owner, person, presence, or legal-admissibility
//! claim. Protocol and identifier-kind values are explicit host declarations;
//! unknown time and unknown protocol remain explicit. Bytes are caller-buffered.

use core::num::{NonZeroU32, NonZeroU64, NonZeroU8};

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

use super::{
    radio_evidence::{RadioEvidenceReference, RadioObservationTime, RadioPermissionContext},
    radio_receipt::ReceiptTimestamp,
};

pub const NFC_EVIDENCE_VERSION: u16 = 1;
pub const NFC_EVIDENCE_RECORD_SIZE: usize = 320;
pub const NFC_RECEIPT_ENCODED_LEN: usize = 240;
pub const NFC_IDENTIFIER_MAX_LEN: usize = 32;
const NFC_MAGIC: [u8; 4] = *b"NFC1";
const RECEIPT_DOMAIN: &[u8] = b"qualia:variable-radio-custody:v1\0";
const CHECKPOINT_DOMAIN: &[u8] = b"qualia:variable-radio-checkpoint:v1\0";

#[path = "nfc_receipt_wire.rs"]
mod receipt_wire;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VariableRadioIdentifierKind {
    NfcUid,
    ProtocolIdentifier,
    OpaqueProtocolValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolIdentifier {
    Unknown,
    Known(NonZeroU64),
}

impl ProtocolIdentifier {
    pub const fn from_optional(value: u64) -> Self {
        match NonZeroU64::new(value) {
            Some(value) => Self::Known(value),
            None => Self::Unknown,
        }
    }

    pub const fn value(self) -> Option<u64> {
        match self {
            Self::Unknown => None,
            Self::Known(value) => Some(value.get()),
        }
    }
}

/// Exact variable-length identifier bytes. The unused fixed-array suffix is
/// always zero; `len` is authoritative and prevents trimming meaningful zeros.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VariableRadioIdentifier {
    kind: VariableRadioIdentifierKind,
    protocol: ProtocolIdentifier,
    len: NonZeroU8,
    bytes: [u8; NFC_IDENTIFIER_MAX_LEN],
}

impl VariableRadioIdentifier {
    pub fn new(
        kind: VariableRadioIdentifierKind,
        protocol: ProtocolIdentifier,
        value: &[u8],
    ) -> Result<Self, NfcEvidenceError> {
        if value.is_empty() || value.len() > NFC_IDENTIFIER_MAX_LEN {
            return Err(NfcEvidenceError::InvalidIdentifierLength);
        }
        let len =
            NonZeroU8::new(value.len() as u8).ok_or(NfcEvidenceError::InvalidIdentifierLength)?;
        let mut bytes = [0u8; NFC_IDENTIFIER_MAX_LEN];
        bytes[..value.len()].copy_from_slice(value);
        Ok(Self {
            kind,
            protocol,
            len,
            bytes,
        })
    }

    pub const fn kind(self) -> VariableRadioIdentifierKind {
        self.kind
    }
    pub const fn protocol(self) -> ProtocolIdentifier {
        self.protocol
    }
    pub const fn len(self) -> usize {
        self.len.get() as usize
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..self.len()]
    }
}

/// One raw observation with the same source/capture/time/permission context
/// used for MAC/BSSID records, but an independent variable-length identifier.
/// A missing generation remains explicitly unknown: a stream ID and sequence
/// without it do not establish continuity or distinguish stream restarts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NfcEvidenceRecord {
    record_id: NonZeroU64,
    identifier: VariableRadioIdentifier,
    source_id: NonZeroU64,
    capture_id: NonZeroU64,
    stream_id: Option<NonZeroU64>,
    generation: Option<NonZeroU32>,
    sequence: Option<u64>,
    time: RadioObservationTime,
    evidence: RadioEvidenceReference,
    permission: Option<RadioPermissionContext>,
}

impl NfcEvidenceRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        record_id: u64,
        identifier: VariableRadioIdentifier,
        source_id: u64,
        capture_id: u64,
        stream_id: Option<u64>,
        generation: Option<u32>,
        sequence: Option<u64>,
        time: RadioObservationTime,
        evidence: RadioEvidenceReference,
        permission: Option<RadioPermissionContext>,
    ) -> Result<Self, NfcEvidenceError> {
        let record_id = NonZeroU64::new(record_id).ok_or(NfcEvidenceError::InvalidReference)?;
        let source_id = NonZeroU64::new(source_id).ok_or(NfcEvidenceError::InvalidReference)?;
        let capture_id = NonZeroU64::new(capture_id).ok_or(NfcEvidenceError::InvalidReference)?;
        let stream_id = match stream_id {
            Some(value) => Some(NonZeroU64::new(value).ok_or(NfcEvidenceError::InvalidReference)?),
            None => None,
        };
        let generation = match generation {
            Some(value) => Some(NonZeroU32::new(value).ok_or(NfcEvidenceError::InvalidReference)?),
            None => None,
        };
        if stream_id.is_none() && (generation.is_some() || sequence.is_some()) {
            return Err(NfcEvidenceError::InvalidStreamReference);
        }
        if permission.is_some_and(|value| value.source_id() != source_id.get()) {
            return Err(NfcEvidenceError::PermissionSourceMismatch);
        }
        if permission.is_some_and(|value| {
            identifier.kind() == VariableRadioIdentifierKind::NfcUid
                && value.modality() != super::authorization::SensingModality::Nfc
        }) {
            return Err(NfcEvidenceError::PermissionModalityMismatch);
        }
        if matches!(evidence, RadioEvidenceReference::Artifact(bytes) if bytes.iter().all(|v| *v == 0))
        {
            return Err(NfcEvidenceError::InvalidReference);
        }
        Ok(Self {
            record_id,
            identifier,
            source_id,
            capture_id,
            stream_id,
            generation,
            sequence,
            time,
            evidence,
            permission,
        })
    }

    pub const fn record_id(self) -> u64 {
        self.record_id.get()
    }
    pub const fn identifier(self) -> VariableRadioIdentifier {
        self.identifier
    }
    pub const fn source_id(self) -> u64 {
        self.source_id.get()
    }
    pub const fn capture_id(self) -> u64 {
        self.capture_id.get()
    }
    pub const fn stream_id(self) -> Option<u64> {
        match self.stream_id {
            Some(value) => Some(value.get()),
            None => None,
        }
    }
    pub const fn generation(self) -> Option<u32> {
        match self.generation {
            Some(value) => Some(value.get()),
            None => None,
        }
    }
    pub const fn sequence(self) -> Option<u64> {
        self.sequence
    }
    pub const fn time(self) -> RadioObservationTime {
        self.time
    }
    pub const fn evidence(self) -> RadioEvidenceReference {
        self.evidence
    }
    pub const fn permission(self) -> Option<RadioPermissionContext> {
        self.permission
    }
}

/// Encode a single record into a canonical fixed-size caller buffer.
pub fn encode_nfc_evidence_record(
    record: NfcEvidenceRecord,
    out: &mut [u8],
) -> Result<usize, NfcEvidenceError> {
    if out.len() < NFC_EVIDENCE_RECORD_SIZE {
        return Err(NfcEvidenceError::OutputTooSmall {
            required: NFC_EVIDENCE_RECORD_SIZE,
            available: out.len(),
        });
    }
    let mut cursor = 0;
    put(out, &mut cursor, &NFC_MAGIC);
    put(out, &mut cursor, &NFC_EVIDENCE_VERSION.to_le_bytes());
    put(
        out,
        &mut cursor,
        &(NFC_EVIDENCE_RECORD_SIZE as u16).to_le_bytes(),
    );
    put(out, &mut cursor, &record.record_id().to_le_bytes());
    put(out, &mut cursor, &record.source_id().to_le_bytes());
    put(out, &mut cursor, &record.capture_id().to_le_bytes());
    put_optional_u64(out, &mut cursor, record.stream_id());
    put_optional_u32(out, &mut cursor, record.generation());
    put_optional_u64(out, &mut cursor, record.sequence());
    put_time(out, &mut cursor, record.time());
    let identifier = record.identifier();
    put(out, &mut cursor, &[identifier_kind_tag(identifier.kind())]);
    match identifier.protocol().value() {
        Some(value) => {
            put(out, &mut cursor, &[1]);
            put(out, &mut cursor, &value.to_le_bytes());
        }
        None => {
            put(out, &mut cursor, &[0]);
            put(out, &mut cursor, &[0; 8]);
        }
    }
    put(out, &mut cursor, &[identifier.len() as u8]);
    let mut bytes = [0; NFC_IDENTIFIER_MAX_LEN];
    bytes[..identifier.len()].copy_from_slice(identifier.bytes());
    put(out, &mut cursor, &bytes);
    put_evidence(out, &mut cursor, record.evidence());
    put_permission(out, &mut cursor, record.permission());
    if cursor > NFC_EVIDENCE_RECORD_SIZE {
        return Err(NfcEvidenceError::InvalidEncoding);
    }
    out[cursor..NFC_EVIDENCE_RECORD_SIZE].fill(0);
    Ok(NFC_EVIDENCE_RECORD_SIZE)
}

/// Signature validity is distinct from host trust, permission validity, and
/// chain completeness. Receipt construction hashes the exact encoded record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NfcCustodyReceipt {
    record_id: u64,
    source_id: u64,
    capture_id: u64,
    digest: [u8; 32],
    sequence: u64,
    previous: Option<[u8; 32]>,
    timestamp: ReceiptTimestamp,
    signer_key: [u8; 32],
    signature: [u8; 64],
}

impl NfcCustodyReceipt {
    pub fn sign(
        record: NfcEvidenceRecord,
        scratch: &mut [u8],
        sequence: u64,
        previous: Option<[u8; 32]>,
        timestamp: ReceiptTimestamp,
        key: &SigningKey,
    ) -> Result<Self, NfcEvidenceError> {
        validate_chain(sequence, previous)?;
        if previous.is_some_and(|value| value.iter().all(|byte| *byte == 0)) {
            return Err(NfcEvidenceError::InvalidPrevious);
        }
        let len = encode_nfc_evidence_record(record, scratch)?;
        let digest: [u8; 32] = Sha256::digest(&scratch[..len]).into();
        let mut receipt = Self {
            record_id: record.record_id(),
            source_id: record.source_id(),
            capture_id: record.capture_id(),
            digest,
            sequence,
            previous,
            timestamp,
            signer_key: key.verifying_key().to_bytes(),
            signature: [0; 64],
        };
        receipt.signature = key.sign(&receipt.signing_digest()).to_bytes();
        Ok(receipt)
    }

    pub const fn record_digest(self) -> [u8; 32] {
        self.digest
    }
    pub const fn sequence(self) -> u64 {
        self.sequence
    }
    pub const fn previous_checkpoint(self) -> Option<[u8; 32]> {
        self.previous
    }
    pub const fn timestamp(self) -> ReceiptTimestamp {
        self.timestamp
    }
    pub const fn signer_key(self) -> [u8; 32] {
        self.signer_key
    }
    pub const fn signature(self) -> [u8; 64] {
        self.signature
    }

    pub fn verify_signature(
        &self,
        record: NfcEvidenceRecord,
        scratch: &mut [u8],
    ) -> Result<NfcReceiptStatus, NfcEvidenceError> {
        if self.record_id != record.record_id()
            || self.source_id != record.source_id()
            || self.capture_id != record.capture_id()
        {
            return Err(NfcEvidenceError::RecordMismatch);
        }
        let len = encode_nfc_evidence_record(record, scratch)?;
        let digest: [u8; 32] = Sha256::digest(&scratch[..len]).into();
        if digest != self.digest {
            return Err(NfcEvidenceError::RecordMismatch);
        }
        let key = VerifyingKey::from_bytes(&self.signer_key)
            .map_err(|_| NfcEvidenceError::InvalidSignerKey)?;
        key.verify_strict(
            &self.signing_digest(),
            &Signature::from_bytes(&self.signature),
        )
        .map_err(|_| NfcEvidenceError::InvalidSignature)?;
        Ok(NfcReceiptStatus::SignatureValidUntrusted)
    }

    pub fn verify_with_authority(
        &self,
        record: NfcEvidenceRecord,
        scratch: &mut [u8],
        authority: &impl NfcReceiptAuthority,
    ) -> Result<NfcReceiptStatus, NfcEvidenceError> {
        self.verify_signature(record, scratch)?;
        if !authority.signer_authorized(self.source_id, &self.signer_key) {
            return Err(NfcEvidenceError::SignerNotTrusted);
        }
        if !authority.permission_context_valid(&record) {
            return Err(NfcEvidenceError::PermissionNotTrusted);
        }
        let checkpoint = self.checkpoint_digest();
        if !authority.chain_position_valid(
            self.sequence,
            self.previous.as_ref(),
            self.timestamp,
            &checkpoint,
        ) {
            return Err(NfcEvidenceError::ChainNotTrusted);
        }
        Ok(NfcReceiptStatus::VerifiedByAuthority)
    }

    /// Encode a fixed-size signed receipt. Insufficient output leaves it untouched.
    pub fn encode_into(&self, out: &mut [u8]) -> Result<usize, NfcEvidenceError> {
        receipt_wire::encode(self, out)
    }

    /// Parse structure only; signature, timestamp trust, permission, and chain
    /// remain unverified until `verify_with_authority` succeeds.
    pub fn decode_from(bytes: &[u8]) -> Result<Self, NfcEvidenceError> {
        receipt_wire::decode(bytes)
    }

    fn signing_digest(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(RECEIPT_DOMAIN);
        hash.update(self.record_id.to_le_bytes());
        hash.update(self.source_id.to_le_bytes());
        hash.update(self.capture_id.to_le_bytes());
        hash.update(self.digest);
        hash.update(self.sequence.to_le_bytes());
        hash_previous(&mut hash, self.previous);
        hash_timestamp(&mut hash, self.timestamp);
        hash.update(self.signer_key);
        hash.finalize().into()
    }

    fn checkpoint_digest(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(CHECKPOINT_DOMAIN);
        hash.update(self.signing_digest());
        hash.update(self.signature);
        hash.finalize().into()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NfcReceiptStatus {
    SignatureValidUntrusted,
    VerifiedByAuthority,
}

/// Host-owned trust boundary; implementations must check current signer,
/// permission/revocation, and anti-rollback/checkpoint completeness state. The
/// chain callback receives the signed timestamp verbatim and must decide
/// whether `ReceiptTimestamp::Unknown` is acceptable for the requested use.
pub trait NfcReceiptAuthority {
    fn signer_authorized(&self, source_id: u64, key: &[u8; 32]) -> bool;
    fn permission_context_valid(&self, record: &NfcEvidenceRecord) -> bool;
    fn chain_position_valid(
        &self,
        sequence: u64,
        previous: Option<&[u8; 32]>,
        timestamp: ReceiptTimestamp,
        current: &[u8; 32],
    ) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NfcEvidenceError {
    InvalidIdentifierLength,
    InvalidReference,
    InvalidStreamReference,
    PermissionSourceMismatch,
    PermissionModalityMismatch,
    OutputTooSmall { required: usize, available: usize },
    InvalidEncoding,
    InvalidPrevious,
    InvalidChain,
    RecordMismatch,
    InvalidSignerKey,
    InvalidSignature,
    SignerNotTrusted,
    PermissionNotTrusted,
    ChainNotTrusted,
    InvalidTransport,
}

fn identifier_kind_tag(kind: VariableRadioIdentifierKind) -> u8 {
    match kind {
        VariableRadioIdentifierKind::NfcUid => 1,
        VariableRadioIdentifierKind::ProtocolIdentifier => 2,
        VariableRadioIdentifierKind::OpaqueProtocolValue => 3,
    }
}

fn put_time(out: &mut [u8], cursor: &mut usize, time: RadioObservationTime) {
    match time {
        RadioObservationTime::Unknown => {
            put(out, cursor, &[0]);
            put(out, cursor, &[0; 28]);
        }
        RadioObservationTime::QualifiedMonotonic(stamp) => {
            put(out, cursor, &[1]);
            put(out, cursor, &stamp.clock_id().to_le_bytes());
            put(out, cursor, &stamp.restart_epoch().to_le_bytes());
            put(out, cursor, &stamp.ticks_ns().to_le_bytes());
            put(out, cursor, &stamp.uncertainty_ns().to_le_bytes());
        }
        RadioObservationTime::WindowsUtc100ns(ticks) => {
            put(out, cursor, &[2]);
            put(out, cursor, &ticks.to_le_bytes());
            put(out, cursor, &[0; 20]);
        }
        RadioObservationTime::NativeTicks {
            ticks,
            clock_domain_id,
            unit_id,
        } => {
            put(out, cursor, &[3]);
            put(out, cursor, &ticks.to_le_bytes());
            put_optional_u64(out, cursor, clock_domain_id.map(|value| value.get()));
            put_optional_u64(out, cursor, unit_id.map(|value| value.get()));
            put(out, cursor, &[0; 2]);
        }
    }
}

fn put_evidence(out: &mut [u8], cursor: &mut usize, evidence: RadioEvidenceReference) {
    match evidence {
        RadioEvidenceReference::Unknown => {
            put(out, cursor, &[0]);
            put(out, cursor, &[0; 32]);
        }
        RadioEvidenceReference::NotAttached => {
            put(out, cursor, &[1]);
            put(out, cursor, &[0; 32]);
        }
        RadioEvidenceReference::Artifact(bytes) => {
            put(out, cursor, &[2]);
            put(out, cursor, &bytes);
        }
    }
}

fn put_permission(out: &mut [u8], cursor: &mut usize, permission: Option<RadioPermissionContext>) {
    match permission {
        None => {
            put(out, cursor, &[0]);
            put(out, cursor, &[0; 122]);
        }
        Some(value) => {
            put(out, cursor, &[1]);
            put(out, cursor, &value.grant_id());
            put(out, cursor, &value.grant_digest());
            put(out, cursor, &value.actor_id());
            put(out, cursor, &value.source_id().to_le_bytes());
            put(
                out,
                cursor,
                &[modality_tag(value.modality()), action_tag(value.action())],
            );
            put(out, cursor, &value.purpose_hash().to_le_bytes());
            put(out, cursor, &value.checked_at_unix().to_le_bytes());
        }
    }
}

fn modality_tag(value: super::authorization::SensingModality) -> u8 {
    use super::authorization::SensingModality::*;
    match value {
        Wifi => 1,
        Bluetooth => 2,
        Cellular => 3,
        Sdr => 4,
        Radar => 5,
        VisibleCamera => 6,
        NearInfraredCamera => 7,
        ThermalCamera => 8,
        Microphone => 9,
        FieldInstrument => 10,
        Nfc => 11,
    }
}
fn action_tag(value: super::authorization::SensingAction) -> u8 {
    use super::authorization::SensingAction::*;
    match value {
        Acquire => 1,
        Process => 2,
        Store => 3,
        Export => 4,
        Share => 5,
    }
}
fn put_optional_u64(out: &mut [u8], cursor: &mut usize, value: Option<u64>) {
    match value {
        Some(value) => {
            put(out, cursor, &[1]);
            put(out, cursor, &value.to_le_bytes());
        }
        None => {
            put(out, cursor, &[0]);
            put(out, cursor, &[0; 8]);
        }
    }
}
fn put_optional_u32(out: &mut [u8], cursor: &mut usize, value: Option<u32>) {
    match value {
        Some(value) => {
            put(out, cursor, &[1]);
            put(out, cursor, &value.to_le_bytes());
        }
        None => {
            put(out, cursor, &[0]);
            put(out, cursor, &[0; 4]);
        }
    }
}
fn put(out: &mut [u8], cursor: &mut usize, bytes: &[u8]) {
    let end = *cursor + bytes.len();
    out[*cursor..end].copy_from_slice(bytes);
    *cursor = end;
}
fn validate_chain(sequence: u64, previous: Option<[u8; 32]>) -> Result<(), NfcEvidenceError> {
    if (sequence == 0) == previous.is_none() {
        Ok(())
    } else {
        Err(NfcEvidenceError::InvalidChain)
    }
}
fn hash_previous(hash: &mut Sha256, previous: Option<[u8; 32]>) {
    match previous {
        Some(value) => {
            hash.update([1]);
            hash.update(value);
        }
        None => {
            hash.update([0]);
            hash.update([0; 32]);
        }
    }
}
fn hash_timestamp(hash: &mut Sha256, timestamp: ReceiptTimestamp) {
    match timestamp {
        ReceiptTimestamp::Unknown => {
            hash.update([0]);
            hash.update([0; 28]);
        }
        ReceiptTimestamp::UnixSeconds(value) => {
            hash.update([1]);
            hash.update(value.to_le_bytes());
        }
        ReceiptTimestamp::Monotonic(value) => {
            hash.update([2]);
            hash.update(value.clock_id().to_le_bytes());
            hash.update(value.restart_epoch().to_le_bytes());
            hash.update(value.ticks_ns().to_le_bytes());
            hash.update(value.uncertainty_ns().to_le_bytes());
        }
        ReceiptTimestamp::WindowsUtc100ns(value) => {
            hash.update([3]);
            hash.update(value.to_le_bytes());
        }
    }
}

#[cfg(test)]
#[path = "nfc_evidence_tests.rs"]
mod tests;
