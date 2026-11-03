//! Explicit mapping of Etalon radio labels to QualiaDB observation records.
//!
//! Addresses remain unverified labels. Neither mapping nor a recorded policy
//! context establishes ownership, identity, presence or current permission.
use etalon_sense_core::{
    observation::ObservationEnvelope,
    radio::{
        BluetoothAddressKind, RadioIdentifier as EtalonIdentifier, RadioIdentifierObservation,
        RadioObservationTime as EtalonTime,
    },
};
use qualia_core_db::sensing::{
    radio_evidence::{
        AddressAssignmentKind, RadioAddress48, RadioCaptureReference, RadioEvidenceError,
        RadioEvidenceRecord, RadioEvidenceReference, RadioIdentifier, RadioObservationTime,
        RadioPermissionContext,
    },
    record::RecordClockStamp,
};

/// Host must bind the exact identifier to the envelope, capture reference and
/// policy context, including any cross-registry source mapping and action.
pub trait RadioBridgeAuthority {
    fn approve(
        &self,
        record_id: u64,
        envelope: &ObservationEnvelope,
        observed: &RadioIdentifierObservation,
        capture: &RadioCaptureReference,
        evidence: &RadioEvidenceReference,
        permission: &Option<RadioPermissionContext>,
    ) -> bool;

    /// Confirm the shared clock namespace and epoch, and that the clock is
    /// qualified for the declared monotonic stamp and uncertainty. This is a
    /// host assertion, not independent verification or cross-clock sync.
    /// Unknown or native UTC timestamps need no mapping.
    fn confirm_qualified_clock(
        &self,
        _record_id: u64,
        _envelope: &ObservationEnvelope,
        _observed: &RadioIdentifierObservation,
        _capture: &RadioCaptureReference,
    ) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadioBridgeError {
    NotAuthorized,
    UnmappedClock,
    InvalidTime,
    Record(RadioEvidenceError),
}

/// Preserve both original Etalon declarations and the derived record so host
/// context is not discarded. This type does not establish their authenticity.
pub struct BridgedRadioObservation {
    pub envelope: ObservationEnvelope,
    pub observed: RadioIdentifierObservation,
    pub record: RadioEvidenceRecord,
}

pub fn bridge_radio_observation(
    authority: &impl RadioBridgeAuthority,
    record_id: u64,
    envelope: ObservationEnvelope,
    observed: RadioIdentifierObservation,
    capture: RadioCaptureReference,
    evidence: RadioEvidenceReference,
    permission: Option<RadioPermissionContext>,
) -> Result<BridgedRadioObservation, RadioBridgeError> {
    if record_id == 0 {
        return Err(RadioBridgeError::Record(
            RadioEvidenceError::InvalidIdentifier,
        ));
    }
    if !authority.approve(
        record_id,
        &envelope,
        &observed,
        &capture,
        &evidence,
        &permission,
    ) {
        return Err(RadioBridgeError::NotAuthorized);
    }
    let address = RadioAddress48::new(observed.identifier.address().octets());
    let identifier = match observed.identifier {
        EtalonIdentifier::Mac(_) => RadioIdentifier::Mac {
            address,
            assignment: AddressAssignmentKind::Unknown,
        },
        EtalonIdentifier::Bssid(_) => RadioIdentifier::Bssid {
            address,
            assignment: AddressAssignmentKind::Unknown,
        },
        EtalonIdentifier::Bluetooth { kind, .. } => RadioIdentifier::Bluetooth {
            address,
            assignment: match kind {
                BluetoothAddressKind::Unknown => AddressAssignmentKind::Unknown,
                BluetoothAddressKind::Public => AddressAssignmentKind::Public,
                BluetoothAddressKind::Random => AddressAssignmentKind::Random,
            },
        },
    };
    let time = match observed.observed_at {
        EtalonTime::Unknown => RadioObservationTime::Unknown,
        EtalonTime::WindowsUtc100ns(ticks) => RadioObservationTime::WindowsUtc100ns(ticks),
        EtalonTime::Clock(stamp) => {
            if !authority.confirm_qualified_clock(record_id, &envelope, &observed, &capture) {
                return Err(RadioBridgeError::UnmappedClock);
            }
            let clock = RecordClockStamp::new(
                stamp.clock_id(),
                stamp.restart_epoch(),
                stamp.ticks_ns(),
                stamp.uncertainty_ns(),
            )
            .map_err(|_| RadioBridgeError::InvalidTime)?;
            RadioObservationTime::QualifiedMonotonic(clock)
        }
    };
    let record =
        RadioEvidenceRecord::new(record_id, identifier, capture, time, evidence, permission)
            .map_err(RadioBridgeError::Record)?;
    Ok(BridgedRadioObservation {
        envelope,
        observed,
        record,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use etalon_sense_core::{
        evidence::EvidenceOrigin,
        observation::{
            HostPolicyDecision, MeasurementUncertainty, ObservationKind, QuantityUnitIds,
        },
        quality::{
            CalibrationRecord, CalibrationState, QualityRecord, SourceDescriptor, SourceKind,
        },
        radio::RadioAddress48 as EtalonAddress,
        time::ClockStamp,
    };

    struct Authority {
        approve: bool,
        clock: bool,
    }
    impl RadioBridgeAuthority for Authority {
        fn approve(
            &self,
            record_id: u64,
            envelope: &ObservationEnvelope,
            _: &RadioIdentifierObservation,
            capture: &RadioCaptureReference,
            _: &RadioEvidenceReference,
            permission: &Option<RadioPermissionContext>,
        ) -> bool {
            self.approve
                && record_id == 7
                && envelope.source().source_id() == 41
                && capture.source_id() == 123
                && permission.is_none()
        }
        fn confirm_qualified_clock(
            &self,
            record_id: u64,
            envelope: &ObservationEnvelope,
            _: &RadioIdentifierObservation,
            capture: &RadioCaptureReference,
        ) -> bool {
            self.clock
                && record_id == 7
                && envelope.source().source_id() == 41
                && capture.source_id() == 123
        }
    }

    fn envelope() -> ObservationEnvelope {
        ObservationEnvelope::new(
            ObservationKind::CachedQuery,
            SourceDescriptor::new(41, SourceKind::Live).unwrap(),
            None,
            None,
            None,
            None,
            QuantityUnitIds::new(None, None).unwrap(),
            MeasurementUncertainty::Unknown,
            QualityRecord::Unknown,
            CalibrationRecord::new(CalibrationState::Missing).unwrap(),
            EvidenceOrigin::Unknown,
            None,
            HostPolicyDecision::NotEvaluated,
        )
        .unwrap()
    }

    fn capture() -> RadioCaptureReference {
        RadioCaptureReference::new(123, 9, None, None, None).unwrap()
    }

    fn bridge(
        authority: &Authority,
        record_id: u64,
        observed: RadioIdentifierObservation,
    ) -> Result<BridgedRadioObservation, RadioBridgeError> {
        bridge_radio_observation(
            authority,
            record_id,
            envelope(),
            observed,
            capture(),
            RadioEvidenceReference::Unknown,
            None,
        )
    }

    #[test]
    fn radio_address_namespace_and_declared_assignment_are_preserved() {
        let etalon = EtalonAddress::from_octets([0, 1, 2, 3, 4, 255]);
        let qualia = RadioAddress48::new([0, 1, 2, 3, 4, 255]);
        let authority = Authority {
            approve: true,
            clock: false,
        };
        for (identifier, expected) in [
            (
                EtalonIdentifier::Mac(etalon),
                RadioIdentifier::Mac {
                    address: qualia,
                    assignment: AddressAssignmentKind::Unknown,
                },
            ),
            (
                EtalonIdentifier::Bssid(etalon),
                RadioIdentifier::Bssid {
                    address: qualia,
                    assignment: AddressAssignmentKind::Unknown,
                },
            ),
            (
                EtalonIdentifier::Bluetooth {
                    address: etalon,
                    kind: BluetoothAddressKind::Unknown,
                },
                RadioIdentifier::Bluetooth {
                    address: qualia,
                    assignment: AddressAssignmentKind::Unknown,
                },
            ),
            (
                EtalonIdentifier::Bluetooth {
                    address: etalon,
                    kind: BluetoothAddressKind::Public,
                },
                RadioIdentifier::Bluetooth {
                    address: qualia,
                    assignment: AddressAssignmentKind::Public,
                },
            ),
            (
                EtalonIdentifier::Bluetooth {
                    address: etalon,
                    kind: BluetoothAddressKind::Random,
                },
                RadioIdentifier::Bluetooth {
                    address: qualia,
                    assignment: AddressAssignmentKind::Random,
                },
            ),
        ] {
            let observed = RadioIdentifierObservation {
                identifier,
                observed_at: EtalonTime::Unknown,
            };
            let output = bridge(&authority, 7, observed).unwrap();
            assert_eq!(output.record.identifier(), expected);
            assert_eq!(output.record.time(), RadioObservationTime::Unknown);
            assert_eq!(output.observed, observed);
            assert_eq!(output.envelope.source().source_id(), 41);
            assert_eq!(output.record.capture().source_id(), 123);
        }
    }

    #[test]
    fn clock_requires_explicit_namespace_and_utc_remains_raw() {
        let address = EtalonAddress::from_octets([0; 6]);
        let stamp = ClockStamp::new(1, 2, 3, 1).unwrap();
        let mut observed = RadioIdentifierObservation {
            identifier: EtalonIdentifier::Mac(address),
            observed_at: EtalonTime::Clock(stamp),
        };
        assert!(matches!(
            bridge(
                &Authority {
                    approve: true,
                    clock: false
                },
                7,
                observed
            ),
            Err(RadioBridgeError::UnmappedClock)
        ));
        let result = bridge(
            &Authority {
                approve: true,
                clock: true,
            },
            7,
            observed,
        )
        .unwrap();
        assert_eq!(
            result.record.time(),
            RadioObservationTime::QualifiedMonotonic(RecordClockStamp::new(1, 2, 3, 1).unwrap())
        );
        observed.observed_at = EtalonTime::WindowsUtc100ns(-7);
        let result = bridge(
            &Authority {
                approve: true,
                clock: false,
            },
            7,
            observed,
        )
        .unwrap();
        assert_eq!(
            result.record.time(),
            RadioObservationTime::WindowsUtc100ns(-7)
        );
    }

    #[test]
    fn record_and_cross_namespace_source_binding_are_approved() {
        let observed = RadioIdentifierObservation {
            identifier: EtalonIdentifier::Mac(EtalonAddress::from_octets([255; 6])),
            observed_at: EtalonTime::Unknown,
        };
        assert!(matches!(
            bridge(
                &Authority {
                    approve: false,
                    clock: true
                },
                7,
                observed
            ),
            Err(RadioBridgeError::NotAuthorized)
        ));
        assert!(matches!(
            bridge(
                &Authority {
                    approve: true,
                    clock: true
                },
                8,
                observed
            ),
            Err(RadioBridgeError::NotAuthorized)
        ));
        assert!(matches!(
            bridge(
                &Authority {
                    approve: true,
                    clock: true
                },
                0,
                observed
            ),
            Err(RadioBridgeError::Record(
                RadioEvidenceError::InvalidIdentifier
            ))
        ));
    }
}
