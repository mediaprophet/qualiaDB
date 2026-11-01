//! Explicit NFC observation mapping; UIDs are unverified observation bytes.
use etalon_sense_nfc::NfcObservation;
use qualia_core_db::sensing::{
    nfc_evidence::{
        NfcEvidenceError, NfcEvidenceRecord, ProtocolIdentifier, VariableRadioIdentifier,
        VariableRadioIdentifierKind,
    },
    radio_evidence::{RadioEvidenceReference, RadioObservationTime, RadioPermissionContext},
    record::RecordClockStamp,
};

/// Cross-registry identifiers are supplied explicitly by the host. The
/// original NFC observation is retained, including gap and tag declarations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NfcMapping {
    pub record_id: u64,
    pub source_id: u64,
    pub capture_id: u64,
    pub stream_id: Option<u64>,
    pub generation: Option<u32>,
    pub sequence: Option<u64>,
    pub protocol: ProtocolIdentifier,
    pub evidence: RadioEvidenceReference,
    pub permission: Option<RadioPermissionContext>,
}

/// Host must authorize the precise observation, mapping and policy context.
/// A recorded permission context is historical, not a current grant.
pub trait NfcBridgeAuthority {
    fn approve(&self, observation: &NfcObservation, mapping: &NfcMapping) -> bool;
    /// Confirm shared clock namespace/epoch and qualification of the declared
    /// timestamp and uncertainty; this is a host assertion, not verification.
    fn confirm_qualified_clock(
        &self,
        _observation: &NfcObservation,
        _mapping: &NfcMapping,
    ) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NfcBridgeError {
    NotAuthorized,
    UnmappedClock,
    InvalidTime,
    Record(NfcEvidenceError),
}

/// Original data and mapped record are retained together without attributing
/// an observed UID to a person, animal, agent, or authenticated device.
pub struct BridgedNfcObservation {
    pub observation: NfcObservation,
    pub record: NfcEvidenceRecord,
}

pub fn bridge_nfc_observation(
    authority: &impl NfcBridgeAuthority,
    observation: NfcObservation,
    mapping: NfcMapping,
) -> Result<BridgedNfcObservation, NfcBridgeError> {
    if !authority.approve(&observation, &mapping) {
        return Err(NfcBridgeError::NotAuthorized);
    }
    let identifier = VariableRadioIdentifier::new(
        VariableRadioIdentifierKind::NfcUid,
        mapping.protocol,
        observation.uid(),
    )
    .map_err(NfcBridgeError::Record)?;
    let time = match observation.metadata().timestamp {
        None => RadioObservationTime::Unknown,
        Some(stamp) => {
            if !authority.confirm_qualified_clock(&observation, &mapping) {
                return Err(NfcBridgeError::UnmappedClock);
            }
            RadioObservationTime::QualifiedMonotonic(
                RecordClockStamp::new(
                    stamp.clock_id(),
                    stamp.restart_epoch(),
                    stamp.ticks_ns(),
                    stamp.uncertainty_ns(),
                )
                .map_err(|_| NfcBridgeError::InvalidTime)?,
            )
        }
    };
    let record = NfcEvidenceRecord::new(
        mapping.record_id,
        identifier,
        mapping.source_id,
        mapping.capture_id,
        mapping.stream_id,
        mapping.generation,
        mapping.sequence,
        time,
        mapping.evidence,
        mapping.permission,
    )
    .map_err(NfcBridgeError::Record)?;
    Ok(BridgedNfcObservation {
        observation,
        record,
    })
}

#[cfg(test)]
#[path = "nfc_tests.rs"]
mod tests;
