//! Fixed v1 wire encoding for signed sensing evidence.
//!
//! Parsing checks structure only. A successfully decoded envelope remains
//! untrusted until its signature and caller authorization policy are verified.

use super::super::grounding::{EvidenceUncertainty, SensingEvidence};
use super::super::record::{CalibrationArtifactReference, CalibrationDeclaration};
use super::{
    all_zero, calibration_reference_matches, calibration_tag, CaptureTimestamp,
    SignedEvidenceError, SignedSensingEvidence, SIGNED_SENSING_EVIDENCE_VERSION,
    SIGNED_SENSING_EVIDENCE_WIRE_SIZE,
};
use crate::NQuin;

const WIRE_MAGIC: [u8; 4] = *b"SSE1";

pub(super) fn encode(
    envelope: &SignedSensingEvidence,
    out: &mut [u8],
) -> Result<usize, SignedEvidenceError> {
    if out.len() < SIGNED_SENSING_EVIDENCE_WIRE_SIZE {
        return Err(SignedEvidenceError::OutputTooSmall);
    }
    let mut cursor = 0;
    put(out, &mut cursor, &WIRE_MAGIC);
    put_u16(out, &mut cursor, envelope.version);
    put_u16(out, &mut cursor, SIGNED_SENSING_EVIDENCE_WIRE_SIZE as u16);
    let e = envelope.evidence;
    let q = e.assertion();
    put_u64(out, &mut cursor, e.citation_id());
    put_u64(out, &mut cursor, e.source_id());
    put_u64(out, &mut cursor, e.sample_id());
    put_u64(out, &mut cursor, q.subject);
    put_u64(out, &mut cursor, q.predicate);
    put_u64(out, &mut cursor, q.object);
    put_u64(out, &mut cursor, q.context);
    put_u64(out, &mut cursor, q.metadata);
    put_u64(out, &mut cursor, q.parity);
    put_u64(out, &mut cursor, e.physical_quantity_id());
    put_u64(out, &mut cursor, e.unit_id());
    match e.uncertainty() {
        EvidenceUncertainty::Unknown => {
            put(out, &mut cursor, &[0]);
            put_u32(out, &mut cursor, 0);
        }
        EvidenceUncertainty::RelativeMillionths(value) => {
            put(out, &mut cursor, &[1]);
            put_u32(out, &mut cursor, value);
        }
    }
    put(out, &mut cursor, &[calibration_tag(e.calibration())]);
    put(out, &mut cursor, &envelope.calibration_reference_id);
    put(out, &mut cursor, &envelope.grant_id);
    put(out, &mut cursor, &envelope.grant_digest);
    put(out, &mut cursor, &envelope.decision_commitment);
    put_u64(out, &mut cursor, envelope.adapter_id);
    put_u32(out, &mut cursor, envelope.adapter_epoch);
    put_u64(out, &mut cursor, envelope.capture_time.clock_domain_id);
    put_u32(out, &mut cursor, envelope.capture_time.restart_epoch);
    put_u64(out, &mut cursor, envelope.capture_time.ticks_ns);
    put_u64(out, &mut cursor, envelope.capture_time.uncertainty_ns);
    put(out, &mut cursor, &envelope.calibration_artifact_commitment);
    put(out, &mut cursor, &envelope.payload_digest);
    put(out, &mut cursor, &envelope.signature);
    debug_assert_eq!(cursor, SIGNED_SENSING_EVIDENCE_WIRE_SIZE);
    Ok(cursor)
}

pub(super) fn decode(input: &[u8]) -> Result<SignedSensingEvidence, SignedEvidenceError> {
    if input.len() != SIGNED_SENSING_EVIDENCE_WIRE_SIZE {
        return Err(SignedEvidenceError::InvalidWireLength);
    }
    let mut cursor = 0;
    if take_array::<4>(input, &mut cursor)? != WIRE_MAGIC {
        return Err(SignedEvidenceError::InvalidWireHeader);
    }
    let version = take_u16(input, &mut cursor)?;
    if version != SIGNED_SENSING_EVIDENCE_VERSION
        || take_u16(input, &mut cursor)? as usize != SIGNED_SENSING_EVIDENCE_WIRE_SIZE
    {
        return Err(SignedEvidenceError::UnsupportedVersion);
    }
    let citation_id = take_u64(input, &mut cursor)?;
    let source_id = take_u64(input, &mut cursor)?;
    let sample_id = take_u64(input, &mut cursor)?;
    let assertion = NQuin {
        subject: take_u64(input, &mut cursor)?,
        predicate: take_u64(input, &mut cursor)?,
        object: take_u64(input, &mut cursor)?,
        context: take_u64(input, &mut cursor)?,
        metadata: take_u64(input, &mut cursor)?,
        parity: take_u64(input, &mut cursor)?,
    };
    let physical_quantity_id = take_u64(input, &mut cursor)?;
    let unit_id = take_u64(input, &mut cursor)?;
    let uncertainty_tag = take_u8(input, &mut cursor)?;
    let uncertainty_value = take_u32(input, &mut cursor)?;
    let uncertainty = match uncertainty_tag {
        0 if uncertainty_value == 0 => EvidenceUncertainty::Unknown,
        1 => EvidenceUncertainty::relative_millionths(uncertainty_value)
            .map_err(|_| SignedEvidenceError::InvalidEvidence)?,
        _ => return Err(SignedEvidenceError::InvalidEvidence),
    };
    let calibration_tag = take_u8(input, &mut cursor)?;
    let calibration_reference_id = take_array::<32>(input, &mut cursor)?;
    let calibration = match calibration_tag {
        0 if all_zero(&calibration_reference_id) => CalibrationDeclaration::Unknown,
        1 if all_zero(&calibration_reference_id) => CalibrationDeclaration::Missing,
        2 if all_zero(&calibration_reference_id) => CalibrationDeclaration::Expired,
        3 if all_zero(&calibration_reference_id) => CalibrationDeclaration::OutOfRange,
        4 if all_zero(&calibration_reference_id) => CalibrationDeclaration::Uncalibrated,
        5 => CalibrationDeclaration::Calibrated(
            CalibrationArtifactReference::new(calibration_reference_id)
                .map_err(|_| SignedEvidenceError::InvalidCalibrationReference)?,
        ),
        _ => return Err(SignedEvidenceError::InvalidCalibrationReference),
    };
    let evidence = SensingEvidence::new(
        citation_id,
        source_id,
        sample_id,
        assertion,
        physical_quantity_id,
        unit_id,
        uncertainty,
        calibration,
    )
    .map_err(|_| SignedEvidenceError::InvalidEvidence)?;
    if !calibration_reference_matches(&evidence, &calibration_reference_id) {
        return Err(SignedEvidenceError::InvalidCalibrationReference);
    }
    let grant_id = take_array::<32>(input, &mut cursor)?;
    let grant_digest = take_array::<32>(input, &mut cursor)?;
    let decision_commitment = take_array::<32>(input, &mut cursor)?;
    let adapter_id = take_u64(input, &mut cursor)?;
    let adapter_epoch = take_u32(input, &mut cursor)?;
    let capture_time = CaptureTimestamp::new(
        take_u64(input, &mut cursor)?,
        take_u32(input, &mut cursor)?,
        take_u64(input, &mut cursor)?,
        take_u64(input, &mut cursor)?,
    )?;
    let calibration_artifact_commitment = take_array::<32>(input, &mut cursor)?;
    let payload_digest = take_array::<32>(input, &mut cursor)?;
    let signature = take_array::<64>(input, &mut cursor)?;
    if cursor != input.len() {
        return Err(SignedEvidenceError::InvalidWireLength);
    }
    if all_zero(&grant_id) || all_zero(&grant_digest) || all_zero(&decision_commitment) {
        return Err(SignedEvidenceError::InvalidIdentifier);
    }
    if adapter_id == 0 {
        return Err(SignedEvidenceError::InvalidIdentifier);
    }
    if all_zero(&payload_digest) {
        return Err(SignedEvidenceError::InvalidPayloadDigest);
    }
    let calibrated = matches!(calibration, CalibrationDeclaration::Calibrated(_));
    if calibrated != !all_zero(&calibration_artifact_commitment) {
        return Err(SignedEvidenceError::InvalidCalibrationCommitment);
    }
    Ok(SignedSensingEvidence {
        version,
        evidence,
        grant_id,
        grant_digest,
        decision_commitment,
        adapter_id,
        adapter_epoch,
        capture_time,
        calibration_reference_id,
        calibration_artifact_commitment,
        payload_digest,
        signature,
    })
}

fn put(out: &mut [u8], cursor: &mut usize, bytes: &[u8]) {
    let end = *cursor + bytes.len();
    out[*cursor..end].copy_from_slice(bytes);
    *cursor = end;
}

fn put_u16(out: &mut [u8], cursor: &mut usize, value: u16) {
    put(out, cursor, &value.to_le_bytes());
}

fn put_u32(out: &mut [u8], cursor: &mut usize, value: u32) {
    put(out, cursor, &value.to_le_bytes());
}

fn put_u64(out: &mut [u8], cursor: &mut usize, value: u64) {
    put(out, cursor, &value.to_le_bytes());
}

fn take<'a>(
    input: &'a [u8],
    cursor: &mut usize,
    length: usize,
) -> Result<&'a [u8], SignedEvidenceError> {
    let end = (*cursor)
        .checked_add(length)
        .ok_or(SignedEvidenceError::InvalidWireLength)?;
    let bytes = input
        .get(*cursor..end)
        .ok_or(SignedEvidenceError::InvalidWireLength)?;
    *cursor = end;
    Ok(bytes)
}

fn take_array<const N: usize>(
    input: &[u8],
    cursor: &mut usize,
) -> Result<[u8; N], SignedEvidenceError> {
    take(input, cursor, N)?
        .try_into()
        .map_err(|_| SignedEvidenceError::InvalidWireLength)
}

fn take_u8(input: &[u8], cursor: &mut usize) -> Result<u8, SignedEvidenceError> {
    Ok(take(input, cursor, 1)?[0])
}

fn take_u16(input: &[u8], cursor: &mut usize) -> Result<u16, SignedEvidenceError> {
    Ok(u16::from_le_bytes(take_array(input, cursor)?))
}

fn take_u32(input: &[u8], cursor: &mut usize) -> Result<u32, SignedEvidenceError> {
    Ok(u32::from_le_bytes(take_array(input, cursor)?))
}

fn take_u64(input: &[u8], cursor: &mut usize) -> Result<u64, SignedEvidenceError> {
    Ok(u64::from_le_bytes(take_array(input, cursor)?))
}
