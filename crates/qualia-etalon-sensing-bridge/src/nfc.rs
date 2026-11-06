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

impl BridgedNfcObservation {
    /// Emit 4 canonical 48-byte `NQuin`s (with 5-field XOR parity) binding the NFC observation
    /// strictly to `Plane::Instrument` (`InstrumentKind::SensorId`) and `Plane::Handle`
    /// (`EnvironmentPredicateShape`), never `Plane::Entity` (`NaturalAgent`).
    pub fn enumerate_fabric_quins(&self, out: &mut [qualia_core_db::NQuin; 4]) -> usize {
        use qualia_core_db::net::qdnf::{authority::Plane, fabric::InstrumentKind};
        use qualia_core_db::{q_hash, NQuin};

        let mut uid_word = [0u8; 8];
        for (idx, &b) in self.record.identifier().bytes().iter().take(8).enumerate() {
            uid_word[idx] = b;
        }
        let uid_payload = u64::from_le_bytes(uid_word) & 0x0FFF_FFFF_FFFF_FFFF;

        let mk_quin = |s: u64, p: u64, o: u64, c: u64, m: u64| NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: c,
            metadata: m,
            parity: NQuin::calculate_parity(s, p, o, c, m),
        };

        out[0] = mk_quin(
            self.record.source_id(),
            q_hash("urn:qualia:idf:instrumentKind"),
            q_hash("urn:qualia:idf:instrument:SensorId"),
            q_hash("urn:qualia:idf:plane:Instrument"),
            ((Plane::Instrument as u64) << 56) | (InstrumentKind::SensorId as u64),
        );
        out[1] = mk_quin(
            self.record.record_id(),
            q_hash("urn:qualia:sensing:nfc:observedUidInstrument"),
            uid_payload,
            q_hash("urn:qualia:idf:plane:Instrument"),
            self.record.identifier().len() as u64,
        );
        out[2] = mk_quin(
            self.record.capture_id(),
            q_hash("urn:qualia:idf:spatiotemporalWindow"),
            q_hash("urn:qualia:idf:shape:EnvironmentPredicateShape"),
            q_hash("urn:qualia:idf:plane:Handle"),
            self.record.sequence().unwrap_or(0),
        );
        out[3] = mk_quin(
            self.record.record_id(),
            q_hash("urn:qualia:etalon:spectrum:coverage"),
            q_hash("urn:qualia:etalon:spectrum:RadioRf"),
            q_hash("urn:qualia:idf:plane:Claim"),
            (1u64 << 63) | 13_560_000u64,
        );
        4
    }

    /// Attempt to join an observed NFC UID directly to a `NaturalAgent` (`Plane::Entity`).
    /// Strictly fails closed with `QdnfError::Denied`.
    pub fn attempt_natural_agent_join(
        &self,
        _target_agent_hash: u64,
    ) -> Result<(), qualia_core_db::net::qdnf::errors::QdnfError> {
        use qualia_core_db::net::qdnf::{
            authority::Plane,
            fabric::{attempt_join, JoinAttempt, JoinKey},
        };
        attempt_join(JoinAttempt {
            left: Plane::Instrument,
            right: Plane::Entity,
            key: JoinKey::SensorCoPresence,
        })
    }
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

#[cfg(test)]
mod fabric_plane_tests {
    use super::*;
    use etalon_sense_nfc::{HostNfcMetadata, NfcProtocol, NfcProximity, NfcTagKind};
    use qualia_core_db::net::qdnf::errors::QdnfError;
    use qualia_core_db::{q_hash, NQuin};

    struct ApproveNfc;
    impl NfcBridgeAuthority for ApproveNfc {
        fn approve(&self, _observation: &NfcObservation, _mapping: &NfcMapping) -> bool {
            true
        }
    }

    #[test]
    fn nfc_uid_emits_instrument_and_handle_quins_and_never_joins_natural_agent() {
        let metadata = HostNfcMetadata {
            source_id: 101,
            stream_id: 5,
            generation: 1,
            sequence: 9,
            gap_before: None,
            uid_len: 7,
            protocol: NfcProtocol::Iso14443A,
            tag_kind: NfcTagKind::Type4,
            proximity: NfcProximity::HostReportedPresent,
            timestamp: None,
        };
        let obs =
            NfcObservation::new(metadata, &[0x04, 0xA1, 0xB2, 0xC3, 0xD4, 0xE5, 0xF6]).unwrap();
        let mapping = NfcMapping {
            record_id: 501,
            source_id: 601,
            capture_id: 701,
            stream_id: Some(801),
            generation: Some(1),
            sequence: Some(9),
            protocol: ProtocolIdentifier::from_optional(1),
            evidence: RadioEvidenceReference::Unknown,
            permission: None,
        };
        let bridged = bridge_nfc_observation(&ApproveNfc, obs, mapping).expect("bridge nfc");
        let zero_quin = NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let mut quins = [zero_quin; 4];
        assert_eq!(bridged.enumerate_fabric_quins(&mut quins), 4);
        for (idx, q) in quins.iter().enumerate() {
            assert!(
                q.verify_ecc_parity(),
                "NFC fabric quin {idx} must have valid 5-field XOR parity"
            );
            assert_ne!(
                q.context,
                q_hash("urn:qualia:idf:plane:Entity"),
                "NFC observation must never emit on the NaturalAgent / Entity plane"
            );
        }
        assert_eq!(quins[0].context, q_hash("urn:qualia:idf:plane:Instrument"));
        assert_eq!(quins[1].context, q_hash("urn:qualia:idf:plane:Instrument"));
        assert_eq!(quins[2].context, q_hash("urn:qualia:idf:plane:Handle"));

        let join_err = bridged
            .attempt_natural_agent_join(q_hash("urn:qualia:entity:natural-agent:alice"))
            .expect_err("NFC UID cannot be joined to a NaturalAgent");
        assert_eq!(join_err, QdnfError::Denied);
    }
}
