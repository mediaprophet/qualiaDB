//! Caller-buffered records for observation-specific radio identifiers.
//!
//! Raw MAC/BSSID/Bluetooth addresses are preserved as six octets. They are not
//! people, owners, authenticated device identities, or evidence of presence.
//! Address assignment kind is host/OS-declared and is never inferred from the
//! address bits. Permission context records a fresh Qualia sensing decision's
//! scope; it does not itself enforce future use or establish a data-subject or
//! guardianship relationship. Records and Q42 projections are not claims of
//! police admissibility or legal sufficiency.

use core::num::{NonZeroU32, NonZeroU64};

use crate::{
    foundation::frame_layout::{self, INLINE_TAG_INTEGER, INLINE_VALUE_MASK},
    NQuin,
};

use super::{
    authorization::{RevalidatedSensingUse, SensingAction, SensingModality},
    record::RecordClockStamp,
};

#[path = "radio_evidence_wire.rs"]
mod wire;
pub use wire::encode_radio_evidence_records;

pub const RADIO_EVIDENCE_VERSION: u16 = 1;
pub const RADIO_EVIDENCE_RECORD_SIZE: usize = 256;
pub const MAX_RADIO_EVIDENCE_RECORDS: usize = 256;

/// Proposed semantic properties. These identifiers are new proposals, not
/// members of an existing registered radio vocabulary.
pub const PROPOSED_RADIO_ADDRESS48_PREDICATE: u64 =
    crate::q_hash("urn:qualia:proposed:sensing:radioAddress48");
pub const PROPOSED_RADIO_PROTOCOL_PREDICATE: u64 =
    crate::q_hash("urn:qualia:proposed:sensing:radioProtocol");
pub const PROPOSED_RADIO_ASSIGNMENT_PREDICATE: u64 =
    crate::q_hash("urn:qualia:proposed:sensing:radioAddressAssignment");
pub const PROPOSED_RADIO_VOCABULARY_STATUS: &str = "proposed; not registered";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RadioAddress48([u8; 6]);

impl RadioAddress48 {
    /// Input octets are preserved in network/MSB-first display order.
    pub const fn new(octets_msb_first: [u8; 6]) -> Self {
        Self(octets_msb_first)
    }

    pub const fn octets_msb_first(self) -> [u8; 6] {
        self.0
    }

    /// The 48-bit unsigned value in the same octet order, suitable for the
    /// canonical inline xsd:integer object tag.
    pub const fn value_u64(self) -> u64 {
        ((self.0[0] as u64) << 40)
            | ((self.0[1] as u64) << 32)
            | ((self.0[2] as u64) << 24)
            | ((self.0[3] as u64) << 16)
            | ((self.0[4] as u64) << 8)
            | (self.0[5] as u64)
    }
}

/// Address allocation type as declared by the host/OS. No bit-pattern
/// heuristic is applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressAssignmentKind {
    Unknown,
    Public,
    Random,
}

/// Observation-level protocol tag; this does not assert device ownership.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadioIdentifier {
    Mac {
        address: RadioAddress48,
        assignment: AddressAssignmentKind,
    },
    Bssid {
        address: RadioAddress48,
        assignment: AddressAssignmentKind,
    },
    Bluetooth {
        address: RadioAddress48,
        assignment: AddressAssignmentKind,
    },
}

impl RadioIdentifier {
    pub const fn address(self) -> RadioAddress48 {
        match self {
            Self::Mac { address, .. }
            | Self::Bssid { address, .. }
            | Self::Bluetooth { address, .. } => address,
        }
    }

    pub const fn assignment(self) -> AddressAssignmentKind {
        match self {
            Self::Mac { assignment, .. }
            | Self::Bssid { assignment, .. }
            | Self::Bluetooth { assignment, .. } => assignment,
        }
    }

    const fn protocol_tag(self) -> u64 {
        match self {
            Self::Mac { .. } => 1,
            Self::Bssid { .. } => 2,
            Self::Bluetooth { .. } => 3,
        }
    }
}

/// Time attached to this observation. Native OS ticks remain in their
/// declared representation until a separately qualified mapping is applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadioObservationTime {
    Unknown,
    QualifiedMonotonic(RecordClockStamp),
    /// Windows `DateTime::UniversalTime` ticks, 100 ns from 1601-01-01 UTC.
    WindowsUtc100ns(i64),
    NativeTicks {
        ticks: i64,
        clock_domain_id: Option<NonZeroU64>,
        unit_id: Option<NonZeroU64>,
    },
}

impl RadioObservationTime {
    pub const fn native_ticks(
        ticks: i64,
        clock_domain_id: Option<u64>,
        unit_id: Option<u64>,
    ) -> Result<Self, RadioEvidenceError> {
        let clock_domain_id = match clock_domain_id {
            Some(value) => match NonZeroU64::new(value) {
                Some(value) => Some(value),
                None => return Err(RadioEvidenceError::InvalidIdentifier),
            },
            None => None,
        };
        let unit_id = match unit_id {
            Some(value) => match NonZeroU64::new(value) {
                Some(value) => Some(value),
                None => return Err(RadioEvidenceError::InvalidIdentifier),
            },
            None => None,
        };
        Ok(Self::NativeTicks {
            ticks,
            clock_domain_id,
            unit_id,
        })
    }
}

/// Observation reference is independent of the address. A source is a sensor
/// declaration, not the observed radio device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RadioCaptureReference {
    source_id: NonZeroU64,
    capture_id: NonZeroU64,
    stream_id: Option<NonZeroU64>,
    generation: Option<NonZeroU32>,
    sequence: Option<u64>,
}

impl RadioCaptureReference {
    pub const fn new(
        source_id: u64,
        capture_id: u64,
        stream_id: Option<u64>,
        generation: Option<u32>,
        sequence: Option<u64>,
    ) -> Result<Self, RadioEvidenceError> {
        let source_id = match NonZeroU64::new(source_id) {
            Some(value) => value,
            None => return Err(RadioEvidenceError::InvalidIdentifier),
        };
        let capture_id = match NonZeroU64::new(capture_id) {
            Some(value) => value,
            None => return Err(RadioEvidenceError::InvalidIdentifier),
        };
        let stream_id = match stream_id {
            Some(value) => match NonZeroU64::new(value) {
                Some(value) => Some(value),
                None => return Err(RadioEvidenceError::InvalidIdentifier),
            },
            None => None,
        };
        let generation = match generation {
            Some(value) => match NonZeroU32::new(value) {
                Some(value) => Some(value),
                None => return Err(RadioEvidenceError::InvalidIdentifier),
            },
            None => None,
        };
        // Missing generation is unknown, not a restart discriminator. A host
        // must not infer continuity from stream ID and sequence alone.
        if stream_id.is_none() && (generation.is_some() || sequence.is_some()) {
            return Err(RadioEvidenceError::InvalidCaptureReference);
        }
        Ok(Self {
            source_id,
            capture_id,
            stream_id,
            generation,
            sequence,
        })
    }

    pub const fn source_id(self) -> u64 {
        self.source_id.get()
    }
    pub const fn capture_id(self) -> u64 {
        self.capture_id.get()
    }
    pub const fn stream_id(self) -> Option<u64> {
        match self.stream_id {
            Some(v) => Some(v.get()),
            None => None,
        }
    }
    pub const fn generation(self) -> Option<u32> {
        match self.generation {
            Some(v) => Some(v.get()),
            None => None,
        }
    }
    pub const fn sequence(self) -> Option<u64> {
        self.sequence
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadioEvidenceReference {
    Unknown,
    NotAttached,
    Artifact([u8; 32]),
}

/// Permission context copied from a freshly revalidated Qualia sensing use.
/// It records actor/grant/request scope, never the observed address owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RadioPermissionContext {
    grant_id: [u8; 32],
    grant_digest: [u8; 32],
    actor_id: [u8; 32],
    source_id: NonZeroU64,
    modality: SensingModality,
    action: SensingAction,
    purpose_hash: NonZeroU64,
    checked_at_unix: u64,
}

impl RadioPermissionContext {
    pub fn from_revalidated(
        use_context: &RevalidatedSensingUse,
    ) -> Result<Self, RadioEvidenceError> {
        let request = use_context.request();
        let source_id = NonZeroU64::new(request.source_id())
            .ok_or(RadioEvidenceError::InvalidPermissionContext)?;
        let purpose_hash = NonZeroU64::new(request.purpose_hash())
            .ok_or(RadioEvidenceError::InvalidPermissionContext)?;
        let grant_id = use_context.grant_id();
        let grant_digest = use_context.grant_digest();
        let actor_id = request.actor_id();
        if all_zero(&grant_id) || all_zero(&grant_digest) || all_zero(&actor_id) {
            return Err(RadioEvidenceError::InvalidPermissionContext);
        }
        Ok(Self {
            grant_id,
            grant_digest,
            actor_id,
            source_id,
            modality: request.modality(),
            action: request.action(),
            purpose_hash,
            checked_at_unix: use_context.checked_at_unix(),
        })
    }

    pub const fn grant_id(self) -> [u8; 32] {
        self.grant_id
    }
    pub const fn grant_digest(self) -> [u8; 32] {
        self.grant_digest
    }
    pub const fn actor_id(self) -> [u8; 32] {
        self.actor_id
    }
    pub const fn source_id(self) -> u64 {
        self.source_id.get()
    }
    pub const fn modality(self) -> SensingModality {
        self.modality
    }
    pub const fn action(self) -> SensingAction {
        self.action
    }
    pub const fn purpose_hash(self) -> u64 {
        self.purpose_hash.get()
    }
    pub const fn checked_at_unix(self) -> u64 {
        self.checked_at_unix
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RadioEvidenceRecord {
    record_id: NonZeroU64,
    identifier: RadioIdentifier,
    capture: RadioCaptureReference,
    time: RadioObservationTime,
    evidence: RadioEvidenceReference,
    permission: Option<RadioPermissionContext>,
}

impl RadioEvidenceRecord {
    pub fn new(
        record_id: u64,
        identifier: RadioIdentifier,
        capture: RadioCaptureReference,
        time: RadioObservationTime,
        evidence: RadioEvidenceReference,
        permission: Option<RadioPermissionContext>,
    ) -> Result<Self, RadioEvidenceError> {
        let record_id = NonZeroU64::new(record_id).ok_or(RadioEvidenceError::InvalidIdentifier)?;
        if matches!(evidence, RadioEvidenceReference::Artifact(id) if all_zero(&id)) {
            return Err(RadioEvidenceError::InvalidEvidenceReference);
        }
        if permission.is_some_and(|context| context.source_id() != capture.source_id()) {
            return Err(RadioEvidenceError::PermissionSourceMismatch);
        }
        if let Some(context) = permission {
            if matches!(identifier, RadioIdentifier::Bssid { .. })
                && context.modality() != SensingModality::Wifi
            {
                return Err(RadioEvidenceError::PermissionModalityMismatch);
            }
            if matches!(identifier, RadioIdentifier::Bluetooth { .. })
                && context.modality() != SensingModality::Bluetooth
            {
                return Err(RadioEvidenceError::PermissionModalityMismatch);
            }
        }
        Ok(Self {
            record_id,
            identifier,
            capture,
            time,
            evidence,
            permission,
        })
    }

    pub const fn record_id(self) -> u64 {
        self.record_id.get()
    }
    pub const fn identifier(self) -> RadioIdentifier {
        self.identifier
    }
    pub const fn capture(self) -> RadioCaptureReference {
        self.capture
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

/// Caller-provided Q42 event subject and graph context. Both are supplied
/// semantic hashes; this module invents no subject-to-person relationship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RadioQuinMapping {
    record_id: NonZeroU64,
    observation_subject_hash: NonZeroU64,
    context_hash: NonZeroU64,
}

impl RadioQuinMapping {
    pub const fn new(
        record_id: u64,
        observation_subject_hash: u64,
        context_hash: u64,
    ) -> Result<Self, RadioEvidenceError> {
        let record_id = match NonZeroU64::new(record_id) {
            Some(value) => value,
            None => return Err(RadioEvidenceError::InvalidIdentifier),
        };
        let observation_subject_hash = match NonZeroU64::new(observation_subject_hash) {
            Some(value) => value,
            None => return Err(RadioEvidenceError::InvalidIdentifier),
        };
        let context_hash = match NonZeroU64::new(context_hash) {
            Some(value) => value,
            None => return Err(RadioEvidenceError::InvalidIdentifier),
        };
        Ok(Self {
            record_id,
            observation_subject_hash,
            context_hash,
        })
    }
}

/// Materialize three proposed Q42 assertions: address value, radio protocol,
/// and host-declared assignment kind. Numeric values use the canonical inline
/// xsd:integer tag; metadata is zero to avoid role collisions; parity uses the
/// shared frame-layout seal. The vocabulary remains explicitly proposed.
pub fn radio_record_to_quins(
    record: RadioEvidenceRecord,
    mapping: RadioQuinMapping,
    out: &mut [NQuin],
) -> Result<usize, RadioEvidenceError> {
    if out.len() < 3 {
        return Err(RadioEvidenceError::OutputTooSmall {
            required: 3,
            available: out.len(),
        });
    }
    if mapping.record_id != record.record_id {
        return Err(RadioEvidenceError::RecordMappingMismatch);
    }
    let predicates = [
        PROPOSED_RADIO_ADDRESS48_PREDICATE,
        PROPOSED_RADIO_PROTOCOL_PREDICATE,
        PROPOSED_RADIO_ASSIGNMENT_PREDICATE,
    ];
    if predicates[0] == predicates[1]
        || predicates[0] == predicates[2]
        || predicates[1] == predicates[2]
    {
        return Err(RadioEvidenceError::VocabularyCollision);
    }
    let values = [
        record.identifier.address().value_u64(),
        record.identifier.protocol_tag(),
        wire::assignment_tag(record.identifier.assignment()),
    ];
    if values.iter().any(|value| value & !INLINE_VALUE_MASK != 0) {
        return Err(RadioEvidenceError::IdentifierOutOfRange);
    }
    let mut staged = [NQuin::default(); 3];
    for index in 0..3 {
        staged[index] = frame_layout::sealed(NQuin {
            subject: mapping.observation_subject_hash.get(),
            predicate: predicates[index],
            object: INLINE_TAG_INTEGER | values[index],
            context: mapping.context_hash.get(),
            metadata: 0,
            parity: 0,
        });
    }
    out[..3].copy_from_slice(&staged);
    Ok(3)
}

fn all_zero(bytes: &[u8]) -> bool {
    bytes.iter().all(|byte| *byte == 0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadioEvidenceError {
    InvalidIdentifier,
    InvalidCaptureReference,
    InvalidEvidenceReference,
    InvalidPermissionContext,
    PermissionSourceMismatch,
    PermissionModalityMismatch,
    RecordMappingMismatch,
    IdentifierOutOfRange,
    VocabularyCollision,
    OutputTooSmall { required: usize, available: usize },
    TooManyRecords,
    DuplicateRecordId,
    RangeOverflow,
}

#[cfg(test)]
#[path = "radio_evidence_tests.rs"]
mod tests;
