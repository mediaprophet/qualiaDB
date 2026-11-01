// Focused record value types and validation.

use super::super::{PhysicalFieldSemantics, SemanticId};
use crate::container_10d::typed_payloads::{TypedPayload, TypedPayloadResolver};
use core::num::NonZeroU64;

/// Opaque location of bytes in an owning container. The ID is not a digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PayloadReference {
    pub(super) container_id: [u8; 16],
    pub(super) section_id: u32,
    pub(super) offset: u64,
    pub(super) length: u64,
}

impl PayloadReference {
    pub const fn new(
        container_id: [u8; 16],
        section_id: u32,
        offset: u64,
        length: u64,
    ) -> Result<Self, RecordError> {
        if all_zero(&container_id) || section_id == 0 || length == 0 {
            return Err(RecordError::InvalidPayloadReference);
        }
        if offset.checked_add(length).is_none() {
            return Err(RecordError::RangeOverflow);
        }
        Ok(Self {
            container_id,
            section_id,
            offset,
            length,
        })
    }

    pub const fn container_id(self) -> [u8; 16] {
        self.container_id
    }
    pub const fn section_id(self) -> u32 {
        self.section_id
    }
    pub const fn offset(self) -> u64 {
        self.offset
    }
    pub const fn length(self) -> u64 {
        self.length
    }
}

/// Opaque calibration artifact reference. Nonzero bytes are an identifier only,
/// not a verified credential or proof that the calibration is valid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalibrationArtifactReference(pub(super) [u8; 32]);

impl CalibrationArtifactReference {
    pub const fn new(bytes: [u8; 32]) -> Result<Self, RecordError> {
        if all_zero(&bytes) {
            return Err(RecordError::InvalidCalibration);
        }
        Ok(Self(bytes))
    }
}

/// Recorded calibration state. No state implies authenticity or freshness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationDeclaration {
    Unknown,
    Missing,
    Expired,
    OutOfRange,
    Uncalibrated,
    Calibrated(CalibrationArtifactReference),
}

/// Provenance presence is explicit: unknown and confirmed absence differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProvenanceDeclaration {
    Unknown,
    NotAttached,
    Reference(PayloadReference),
}

/// Projection presence is explicit; a reference never embeds coordinate values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionDeclaration {
    Unknown,
    NotProjected,
    Reference(ProjectionReference),
}

/// Structurally bound field semantics and the exact MID2 payload range.
///
/// Construct this directly from the selected `TypedPayloadResolver` result.
/// Validation checks field ID, kind, unit ID, section, offset, and byte length;
/// it does not prove that the caller authenticated the envelope or correctly
/// paired a legacy FIEL descriptor with its MID2 declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedFieldPayloadBinding {
    pub(super) physical: PhysicalFieldSemantics,
    pub(super) raw_payload: PayloadReference,
}

impl ResolvedFieldPayloadBinding {
    /// Physical declaration bound to the selected typed payload.
    pub const fn physical(self) -> PhysicalFieldSemantics {
        self.physical
    }
    pub const fn raw_payload(self) -> PayloadReference {
        self.raw_payload
    }

    /// Select and bind the payload for this physical field through a validated
    /// envelope resolver. `container_id` is an external index key; callers
    /// must bind it to the same admitted envelope before use.
    pub fn new(
        physical: PhysicalFieldSemantics,
        resolver: &TypedPayloadResolver<'_, '_>,
        container_id: [u8; 16],
    ) -> Result<Self, RecordError> {
        let resolved_payload = resolver
            .field(physical.field_id())
            .map_err(|_| RecordError::PayloadNotResolved)?
            .ok_or(RecordError::PayloadNotResolved)?;
        Self::bind_payload(physical, resolved_payload, container_id)
    }

    pub(super) fn bind_payload(
        physical: PhysicalFieldSemantics,
        resolved_payload: TypedPayload<'_>,
        container_id: [u8; 16],
    ) -> Result<Self, RecordError> {
        let typed = resolved_payload.record;
        if typed.field_id != physical.field_id()
            || typed.kind != physical.typed_field_kind()
            || typed.unit_id != physical.typed_unit_id().value().unwrap_or(0)
            || typed.payload_section_id == 0
            || typed.payload_length == 0
            || u64::try_from(resolved_payload.bytes.len()).ok() != Some(typed.payload_length)
        {
            return Err(RecordError::FieldPayloadMismatch);
        }
        let raw_payload = PayloadReference::new(
            container_id,
            typed.payload_section_id,
            typed.payload_offset,
            typed.payload_length,
        )?;
        Ok(Self {
            physical,
            raw_payload,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualityMetrics {
    pub(super) confidence_millionths: u32,
    pub(super) relative_uncertainty_millionths: u32,
}

impl QualityMetrics {
    pub const SCALE: u32 = 1_000_000;

    pub const fn new(
        confidence_millionths: u32,
        relative_uncertainty_millionths: u32,
    ) -> Result<Self, RecordError> {
        if confidence_millionths > Self::SCALE || relative_uncertainty_millionths > Self::SCALE {
            return Err(RecordError::InvalidQuality);
        }
        Ok(Self {
            confidence_millionths,
            relative_uncertainty_millionths,
        })
    }
}

/// Measurement quality is separate from source trust and calibration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementQuality {
    Unknown,
    Known(QualityMetrics),
}

/// Integer timestamp in a named monotonic clock epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordClockStamp {
    pub(super) clock_id: u64,
    pub(super) restart_epoch: u32,
    pub(super) ticks_ns: u64,
    pub(super) uncertainty_ns: u64,
}

impl RecordClockStamp {
    pub const fn new(
        clock_id: u64,
        restart_epoch: u32,
        ticks_ns: u64,
        uncertainty_ns: u64,
    ) -> Result<Self, RecordError> {
        if clock_id == 0 {
            return Err(RecordError::InvalidIdentifier);
        }
        if ticks_ns.checked_add(uncertainty_ns).is_none() {
            return Err(RecordError::InvalidTime);
        }
        Ok(Self {
            clock_id,
            restart_epoch,
            ticks_ns,
            uncertainty_ns,
        })
    }

    pub const fn clock_id(self) -> u64 {
        self.clock_id
    }
    pub const fn restart_epoch(self) -> u32 {
        self.restart_epoch
    }
    pub const fn ticks_ns(self) -> u64 {
        self.ticks_ns
    }
    pub const fn uncertainty_ns(self) -> u64 {
        self.uncertainty_ns
    }
}

/// Reference to a separately stored, explicitly materialized projection.
/// It contains no Tensor10D coordinates and cannot create a projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectionReference {
    pub(super) projection_record_id: u64,
    pub(super) profile_id: NonZeroU64,
    pub(super) domain_id: NonZeroU64,
    pub(super) coordinate_convention_id: NonZeroU64,
}

impl ProjectionReference {
    pub const fn new(
        projection_record_id: u64,
        profile_id: u64,
        domain_id: u64,
        coordinate_convention_id: u64,
    ) -> Result<Self, RecordError> {
        if projection_record_id == 0 {
            return Err(RecordError::InvalidIdentifier);
        }
        let profile_id = match NonZeroU64::new(profile_id) {
            Some(id) => id,
            None => return Err(RecordError::InvalidIdentifier),
        };
        let domain_id = match NonZeroU64::new(domain_id) {
            Some(id) => id,
            None => return Err(RecordError::InvalidIdentifier),
        };
        let coordinate_convention_id = match NonZeroU64::new(coordinate_convention_id) {
            Some(id) => id,
            None => return Err(RecordError::InvalidIdentifier),
        };
        Ok(Self {
            projection_record_id,
            profile_id,
            domain_id,
            coordinate_convention_id,
        })
    }
}

/// Validated metadata index for one physical measurement sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeasurementRecord {
    pub(super) record_id: u64,
    pub(super) source_id: SemanticId,
    pub(super) physical: PhysicalFieldSemantics,
    pub(super) raw_payload: PayloadReference,
    pub(super) provenance: ProvenanceDeclaration,
    pub(super) calibration: CalibrationDeclaration,
    pub(super) quality: MeasurementQuality,
    pub(super) timestamp: Option<RecordClockStamp>,
    pub(super) projection: ProjectionDeclaration,
}

impl MeasurementRecord {
    pub const fn record_id(self) -> u64 {
        self.record_id
    }
    pub const fn source_id(self) -> SemanticId {
        self.source_id
    }
    pub const fn physical(self) -> PhysicalFieldSemantics {
        self.physical
    }
    pub const fn raw_payload(self) -> PayloadReference {
        self.raw_payload
    }
    pub const fn calibration(self) -> CalibrationDeclaration {
        self.calibration
    }
    pub const fn quality(self) -> MeasurementQuality {
        self.quality
    }
    pub const fn timestamp(self) -> Option<RecordClockStamp> {
        self.timestamp
    }
    pub const fn provenance(self) -> ProvenanceDeclaration {
        self.provenance
    }
    pub const fn projection(self) -> ProjectionDeclaration {
        self.projection
    }

    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        record_id: u64,
        source_id: SemanticId,
        field_payload: ResolvedFieldPayloadBinding,
        provenance: ProvenanceDeclaration,
        calibration: CalibrationDeclaration,
        quality: MeasurementQuality,
        timestamp: Option<RecordClockStamp>,
        projection: ProjectionDeclaration,
    ) -> Result<Self, RecordError> {
        if record_id == 0 {
            return Err(RecordError::InvalidIdentifier);
        }
        Ok(Self {
            record_id,
            source_id,
            physical: field_payload.physical,
            raw_payload: field_payload.raw_payload,
            provenance,
            calibration,
            quality,
            timestamp,
            projection,
        })
    }
}

const fn all_zero(bytes: &[u8]) -> bool {
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != 0 {
            return false;
        }
        i += 1;
    }
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordError {
    InvalidEncoding,
    InvalidIdentifier,
    InvalidPayloadReference,
    FieldPayloadMismatch,
    PayloadNotResolved,
    RangeOverflow,
    InvalidCalibration,
    InvalidQuality,
    InvalidTime,
    TooManyRecords,
    DuplicateRecordId,
    OutputTooSmall,
}
