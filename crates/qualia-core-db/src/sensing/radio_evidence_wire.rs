//! Fixed-size caller-buffered encoding for the proposed radio evidence record.

use core::num::NonZeroU64;

use super::super::authorization::{SensingAction, SensingModality};

use super::{
    AddressAssignmentKind, RadioEvidenceError, RadioEvidenceRecord, RadioEvidenceReference,
    RadioIdentifier, RadioObservationTime, RadioPermissionContext, MAX_RADIO_EVIDENCE_RECORDS,
    RADIO_EVIDENCE_RECORD_SIZE, RADIO_EVIDENCE_VERSION,
};

const RADIO_RECORD_MAGIC: [u8; 4] = *b"RDE1";

/// Canonically serialize a caller-owned slice into fixed 256-byte records.
/// Duplicate record IDs and capacity failures are checked before any output
/// bytes are changed. This encoding is proposed and is not a Q42 container or
/// signed evidence format.
pub fn encode_radio_evidence_records(
    records: &[RadioEvidenceRecord],
    out: &mut [u8],
) -> Result<usize, RadioEvidenceError> {
    if records.len() > MAX_RADIO_EVIDENCE_RECORDS {
        return Err(RadioEvidenceError::TooManyRecords);
    }
    for i in 0..records.len() {
        for j in (i + 1)..records.len() {
            if records[i].record_id == records[j].record_id {
                return Err(RadioEvidenceError::DuplicateRecordId);
            }
        }
    }
    let needed = records
        .len()
        .checked_mul(RADIO_EVIDENCE_RECORD_SIZE)
        .ok_or(RadioEvidenceError::RangeOverflow)?;
    if out.len() < needed {
        return Err(RadioEvidenceError::OutputTooSmall {
            required: needed,
            available: out.len(),
        });
    }
    for (index, record) in records.iter().enumerate() {
        let start = index * RADIO_EVIDENCE_RECORD_SIZE;
        encode_one(record, &mut out[start..start + RADIO_EVIDENCE_RECORD_SIZE]);
    }
    Ok(needed)
}

fn encode_one(record: &RadioEvidenceRecord, out: &mut [u8]) {
    let mut cursor = 0usize;
    put(out, &mut cursor, &RADIO_RECORD_MAGIC);
    put(out, &mut cursor, &RADIO_EVIDENCE_VERSION.to_le_bytes());
    put(
        out,
        &mut cursor,
        &(RADIO_EVIDENCE_RECORD_SIZE as u16).to_le_bytes(),
    );
    put(out, &mut cursor, &record.record_id.get().to_le_bytes());
    put(out, &mut cursor, &record.capture.source_id().to_le_bytes());
    put(out, &mut cursor, &record.capture.capture_id().to_le_bytes());
    encode_optional_u64(out, &mut cursor, record.capture.stream_id());
    encode_optional_u32(out, &mut cursor, record.capture.generation());
    encode_optional_u64(out, &mut cursor, record.capture.sequence());
    let (protocol, address, assignment) = match record.identifier {
        RadioIdentifier::Mac {
            address,
            assignment,
        } => (1, address, assignment),
        RadioIdentifier::Bssid {
            address,
            assignment,
        } => (2, address, assignment),
        RadioIdentifier::Bluetooth {
            address,
            assignment,
        } => (3, address, assignment),
    };
    put(
        out,
        &mut cursor,
        &[protocol, assignment_tag(assignment) as u8],
    );
    put(out, &mut cursor, &address.octets_msb_first());
    encode_time(out, &mut cursor, record.time);
    encode_evidence(out, &mut cursor, record.evidence);
    encode_permission(out, &mut cursor, record.permission);
    out[cursor..].fill(0);
}

fn encode_time(out: &mut [u8], cursor: &mut usize, time: RadioObservationTime) {
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
            put(
                out,
                cursor,
                &clock_domain_id.map_or(0, NonZeroU64::get).to_le_bytes(),
            );
            put(
                out,
                cursor,
                &unit_id.map_or(0, NonZeroU64::get).to_le_bytes(),
            );
            put(out, cursor, &[0; 4]);
        }
    }
}

fn encode_evidence(out: &mut [u8], cursor: &mut usize, evidence: RadioEvidenceReference) {
    match evidence {
        RadioEvidenceReference::Unknown => {
            put(out, cursor, &[0]);
            put(out, cursor, &[0; 32]);
        }
        RadioEvidenceReference::NotAttached => {
            put(out, cursor, &[1]);
            put(out, cursor, &[0; 32]);
        }
        RadioEvidenceReference::Artifact(id) => {
            put(out, cursor, &[2]);
            put(out, cursor, &id);
        }
    }
}

fn encode_permission(
    out: &mut [u8],
    cursor: &mut usize,
    permission: Option<RadioPermissionContext>,
) {
    let Some(permission) = permission else {
        put(out, cursor, &[0]);
        put(out, cursor, &[0; 122]);
        return;
    };
    put(out, cursor, &[1]);
    put(out, cursor, &permission.grant_id);
    put(out, cursor, &permission.grant_digest);
    put(out, cursor, &permission.actor_id);
    put(out, cursor, &permission.source_id().to_le_bytes());
    put(
        out,
        cursor,
        &[
            modality_tag(permission.modality),
            action_tag(permission.action),
        ],
    );
    put(out, cursor, &permission.purpose_hash().to_le_bytes());
    put(out, cursor, &permission.checked_at_unix.to_le_bytes());
}

fn encode_optional_u64(out: &mut [u8], cursor: &mut usize, value: Option<u64>) {
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

fn encode_optional_u32(out: &mut [u8], cursor: &mut usize, value: Option<u32>) {
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

fn put(out: &mut [u8], cursor: &mut usize, value: &[u8]) {
    let end = *cursor + value.len();
    out[*cursor..end].copy_from_slice(value);
    *cursor = end;
}

pub(super) const fn assignment_tag(kind: AddressAssignmentKind) -> u64 {
    match kind {
        AddressAssignmentKind::Unknown => 1,
        AddressAssignmentKind::Public => 2,
        AddressAssignmentKind::Random => 3,
    }
}

const fn modality_tag(modality: SensingModality) -> u8 {
    match modality {
        SensingModality::Wifi => 1,
        SensingModality::Bluetooth => 2,
        SensingModality::Cellular => 3,
        SensingModality::Sdr => 4,
        SensingModality::Radar => 5,
        SensingModality::VisibleCamera => 6,
        SensingModality::NearInfraredCamera => 7,
        SensingModality::ThermalCamera => 8,
        SensingModality::Microphone => 9,
        SensingModality::FieldInstrument => 10,
        SensingModality::Nfc => 11,
    }
}

const fn action_tag(action: SensingAction) -> u8 {
    match action {
        SensingAction::Acquire => 1,
        SensingAction::Process => 2,
        SensingAction::Store => 3,
        SensingAction::Export => 4,
        SensingAction::Share => 5,
    }
}
