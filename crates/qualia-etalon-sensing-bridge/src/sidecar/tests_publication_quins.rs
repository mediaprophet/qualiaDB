use super::*;
use crate::store_receipt::{prepare_store_receipt, StoreReceiptError};
use crate::bridge_observation;
use crate::{bridge_observation_with_consent, BridgeError};
use ed25519_dalek::SigningKey;
use etalon_sense_core::evidence::EvidenceOrigin;
use qualia_core_db::governance::consent_contract::ConsentGrant;
use qualia_core_db::sensing::authorization::{
    AuthorizationError, RevocationAuthority, SensingAction, SensingRequest, TrustedUnixClock,
};

#[cfg(not(target_arch = "wasm32"))]
struct DeniedPublication;

#[cfg(not(target_arch = "wasm32"))]
impl qualia_core_db::sensing::sidecar_attestation::SidecarReceiptAuthority for DeniedPublication {
    fn permits(
        &self,
        _: &qualia_core_db::sensing::sidecar_attestation::SidecarReceipt,
        _: &[u8; 768],
        _: &[u8],
    ) -> bool {
        true
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl qualia_core_db::sensing::local_custody::CustodyAuthority for DeniedPublication {
    fn permit_publish(
        &mut self,
        _: &std::path::Path,
        _: &qualia_core_db::sensing::sidecar_attestation::SidecarReceipt,
        _: &[u8; 768],
        _: &[u8],
        _: usize,
    ) -> bool {
        false
    }
    fn permit_read(&mut self, _: &std::path::Path, _: &[u8; 32], _: usize) -> bool {
        false
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl qualia_core_db::sensing::local_custody::chain::ChainAuthority for DeniedPublication {
    fn authenticated_head(
        &mut self,
        _: &std::path::Path,
        _: &[u8; 32],
    ) -> Result<
        qualia_core_db::sensing::local_custody::chain::ChainHead,
        qualia_core_db::sensing::local_custody::chain::ChainAuthorityError,
    > {
        panic!("unauthorized head lookup")
    }
    fn compare_and_commit(
        &mut self,
        _: &std::path::Path,
        _: &[u8; 32],
        _: qualia_core_db::sensing::local_custody::chain::ChainHead,
        _: qualia_core_db::sensing::local_custody::chain::ChainHead,
        _: &qualia_core_db::sensing::sidecar_attestation::SidecarReceipt,
    ) -> qualia_core_db::sensing::local_custody::chain::HeadCommit {
        panic!("unauthorized commit")
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::publish_store::ExactPublicationAuthority for DeniedPublication {
    fn permits_exact(
        &mut self,
        _: &ConsentGrant,
        _: &SensingRequest,
        _: &std::path::Path,
        _: &crate::BridgedObservation,
        _: &qualia_core_db::sensing::sidecar_attestation::SidecarReceipt,
        _: &[u8; 768],
        _: &[u8],
    ) -> bool {
        false
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn denied_publication_stops_before_head_or_storage_access() {
    use crate::publish_store::{prepare_and_publish_store, PublishStoreError, StoreConsent};
    use qualia_core_db::sensing::local_custody::{chain::ChainPublicationError, CustodyError};
    let grant = signed_grant();
    let observation = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
    let mut clock = Clock(60);
    let mut consent = StoreConsent {
        grant: &grant,
        identity: &Identity,
        scope: &SensorScope(true),
        revocations: &Revocations(false),
        clock: &mut clock,
    };
    let result = prepare_and_publish_store(
        &std::env::temp_dir(),
        &observation,
        &[5; 16],
        16,
        2048,
        permit(&grant, SensingAction::Store, 17),
        &mut consent,
        &StoreAuthority(true),
        &mut DeniedPublication,
        [1; 32],
        0,
        None,
        &SigningKey::from_bytes(&[11; 32]),
    );
    assert!(matches!(
        result,
        Err(PublishStoreError::Publication(
            ChainPublicationError::Publication(CustodyError::PermissionDenied)
        ))
    ));
}

#[cfg(not(target_arch = "wasm32"))]
struct SharedClock<'a>(&'a std::cell::Cell<u64>);
#[cfg(not(target_arch = "wasm32"))]
impl TrustedUnixClock for SharedClock<'_> {
    fn now_unix(&mut self) -> Result<u64, AuthorizationError> {
        Ok(self.0.get())
    }
}
#[cfg(not(target_arch = "wasm32"))]
struct SharedRevocation<'a>(&'a std::cell::Cell<bool>);
#[cfg(not(target_arch = "wasm32"))]
impl RevocationAuthority for SharedRevocation<'_> {
    fn is_revoked(&self, _: &[u8; 32], _: &[u8; 32], _: u64) -> Result<bool, AuthorizationError> {
        Ok(self.0.get())
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct ChangingPublication<'a> {
    time: &'a std::cell::Cell<u64>,
    revoked: &'a std::cell::Cell<bool>,
    expire: bool,
}
#[cfg(not(target_arch = "wasm32"))]
impl qualia_core_db::sensing::sidecar_attestation::SidecarReceiptAuthority
    for ChangingPublication<'_>
{
    fn permits(
        &self,
        _: &qualia_core_db::sensing::sidecar_attestation::SidecarReceipt,
        _: &[u8; 768],
        _: &[u8],
    ) -> bool {
        true
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl qualia_core_db::sensing::local_custody::CustodyAuthority for ChangingPublication<'_> {
    fn permit_publish(
        &mut self,
        _: &std::path::Path,
        _: &qualia_core_db::sensing::sidecar_attestation::SidecarReceipt,
        _: &[u8; 768],
        _: &[u8],
        _: usize,
    ) -> bool {
        true
    }
    fn permit_read(&mut self, _: &std::path::Path, _: &[u8; 32], _: usize) -> bool {
        false
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl qualia_core_db::sensing::local_custody::chain::ChainAuthority for ChangingPublication<'_> {
    fn authenticated_head(
        &mut self,
        _: &std::path::Path,
        _: &[u8; 32],
    ) -> Result<
        qualia_core_db::sensing::local_custody::chain::ChainHead,
        qualia_core_db::sensing::local_custody::chain::ChainAuthorityError,
    > {
        panic!("stale permission reached head lookup")
    }
    fn compare_and_commit(
        &mut self,
        _: &std::path::Path,
        _: &[u8; 32],
        _: qualia_core_db::sensing::local_custody::chain::ChainHead,
        _: qualia_core_db::sensing::local_custody::chain::ChainHead,
        _: &qualia_core_db::sensing::sidecar_attestation::SidecarReceipt,
    ) -> qualia_core_db::sensing::local_custody::chain::HeadCommit {
        panic!("stale permission reached commit")
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl crate::publish_store::ExactPublicationAuthority for ChangingPublication<'_> {
    fn permits_exact(
        &mut self,
        _: &ConsentGrant,
        _: &SensingRequest,
        _: &std::path::Path,
        _: &crate::BridgedObservation,
        _: &qualia_core_db::sensing::sidecar_attestation::SidecarReceipt,
        _: &[u8; 768],
        _: &[u8],
    ) -> bool {
        if self.expire {
            self.time.set(100);
        } else {
            self.revoked.set(true);
        }
        true
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn publication_revalidates_after_host_callback_changes_policy_or_time() {
    use crate::publish_store::{prepare_and_publish_store, PublishStoreError, StoreConsent};
    use qualia_core_db::sensing::local_custody::{chain::ChainPublicationError, CustodyError};
    for expire in [false, true] {
        let grant = signed_grant();
        let bridged = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
        let time = std::cell::Cell::new(60);
        let revoked = std::cell::Cell::new(false);
        let mut clock = SharedClock(&time);
        let revocations = SharedRevocation(&revoked);
        let mut consent = StoreConsent {
            grant: &grant,
            identity: &Identity,
            scope: &SensorScope(true),
            revocations: &revocations,
            clock: &mut clock,
        };
        let mut host = ChangingPublication {
            time: &time,
            revoked: &revoked,
            expire,
        };
        let result = prepare_and_publish_store(
            &std::env::temp_dir(),
            &bridged,
            &[5; 16],
            16,
            2048,
            permit(&grant, SensingAction::Store, 17),
            &mut consent,
            &StoreAuthority(true),
            &mut host,
            [1; 32],
            0,
            None,
            &SigningKey::from_bytes(&[11; 32]),
        );
        assert!(matches!(
            result,
            Err(PublishStoreError::Publication(
                ChainPublicationError::Publication(CustodyError::PermissionDenied)
            ))
        ));
    }
}

#[test]
fn zero_commitment_and_payload_substitution_do_not_release_bytes() {
    use qualia_core_db::sensing::sidecar_attestation::SIDECAR_RECEIPT_BYTES;
    let grant = signed_grant();
    let bridged = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
    let key = SigningKey::from_bytes(&[11; 32]);
    let mut sidecar = [0xa5; SIDECAR_SIZE];
    let mut receipt = [0xa5; SIDECAR_RECEIPT_BYTES];
    let zero = prepare_store_receipt(
        &ZeroStoreAuthority,
        permit(&grant, SensingAction::Store, 17),
        &grant,
        &Identity,
        &SensorScope(true),
        &Revocations(false),
        &mut Clock(60),
        &bridged,
        &[5; 16],
        16,
        [1; 32],
        0,
        None,
        &key,
        &mut sidecar,
        &mut receipt,
    );
    assert!(matches!(zero, Err(StoreReceiptError::BindingDenied)));
    let wrong = prepare_store_receipt(
        &StoreAuthority(true),
        permit(&grant, SensingAction::Store, 17),
        &grant,
        &Identity,
        &SensorScope(true),
        &Revocations(false),
        &mut Clock(60),
        &bridged,
        &[4; 16],
        16,
        [1; 32],
        0,
        None,
        &key,
        &mut sidecar,
        &mut receipt,
    );
    assert!(matches!(wrong, Err(StoreReceiptError::BindingDenied)));
    assert!(sidecar.iter().chain(receipt.iter()).all(|&b| b == 0xa5));
}

#[test]
fn store_receipt_failures_leave_both_outputs_unchanged() {
    use qualia_core_db::sensing::sidecar_attestation::SIDECAR_RECEIPT_BYTES;
    let grant = signed_grant();
    let bridged = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
    let key = SigningKey::from_bytes(&[11; 32]);
    for (allow, revoked, sequence) in [(false, false, 0), (true, true, 0), (true, false, 1)] {
        let mut sidecar = [0xa5; SIDECAR_SIZE];
        let mut receipt = [0xa5; SIDECAR_RECEIPT_BYTES];
        let result = prepare_store_receipt(
            &StoreAuthority(allow),
            permit(&grant, SensingAction::Store, 17),
            &grant,
            &Identity,
            &SensorScope(true),
            &Revocations(revoked),
            &mut Clock(60),
            &bridged,
            &[5; 16],
            16,
            [1; 32],
            sequence,
            None,
            &key,
            &mut sidecar,
            &mut receipt,
        );
        if sequence == 1 {
            assert!(matches!(
                result,
                Err(StoreReceiptError::Receipt(
                    qualia_core_db::sensing::sidecar_attestation::SidecarReceiptError::InvalidChain
                ))
            ));
        } else {
            assert!(result.is_err());
        }
        assert!(sidecar.iter().chain(receipt.iter()).all(|&b| b == 0xa5));
    }
}

#[test]
fn sidecar_round_trip_preserves_observation_and_binding() {
    let bridged = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
    let mut bytes = [0; SIDECAR_SIZE];
    encode_sidecar(&bridged, &mut bytes).unwrap();
    let decoded = decode_sidecar(&bytes, binding()).unwrap();
    assert_eq!(decoded.historical_observation(), &observation());
    assert_eq!(decoded.record(), bridged.record());
    assert!(!decoded.evidence_verification_claimed());
    let mut repeated = [0; SIDECAR_SIZE];
    reencode_untrusted_sidecar(&decoded, &mut repeated).unwrap();
    assert_eq!(bytes, repeated);
}

#[test]
fn forged_historical_verification_is_never_restored_as_verified_origin() {
    let bridged = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
    let mut bytes = [0; SIDECAR_SIZE];
    encode_sidecar(&bridged, &mut bytes).unwrap();
    assert_eq!(bytes[416], 2);
    bytes[416] = 3;
    let decoded = decode_sidecar(&bytes, binding()).unwrap();
    assert!(decoded.evidence_verification_claimed());
    assert!(matches!(
        decoded.historical_observation().evidence(),
        EvidenceOrigin::SignaturePresented { .. }
    ));
}

#[test]
fn reserved_bytes_and_short_buffers_fail_closed() {
    let bridged = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
    let mut short = [0x5a; SIDECAR_SIZE - 1];
    assert_eq!(
        encode_sidecar(&bridged, &mut short),
        Err(SidecarError::BufferSize)
    );
    assert!(short.iter().all(|&byte| byte == 0x5a));
    let mut bytes = [0; SIDECAR_SIZE];
    encode_sidecar(&bridged, &mut bytes).unwrap();
    for offset in 670..SIDECAR_SIZE {
        bytes[offset] = 1;
        assert!(decode_sidecar(&bytes, binding()).is_err());
        bytes[offset] = 0;
    }
}

#[test]
fn inconsistent_record_and_observation_are_rejected() {
    let bridged = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
    let mut original = [0; SIDECAR_SIZE];
    encode_sidecar(&bridged, &mut original).unwrap();
    for (offset, value) in [(316, 101), (383, 2), (122, 1), (192, 1), (230, 1)] {
        let mut altered = original;
        altered[offset] = value;
        assert!(
            decode_sidecar(&altered, binding()).is_err(),
            "offset {offset}"
        );
    }
}

#[test]
fn unknown_mapped_source_is_rejected() {
    let bridged = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
    let mut bytes = [0u8; SIDECAR_SIZE];
    encode_sidecar(&bridged, &mut bytes).unwrap();
    bytes[24..32].fill(0);
    assert!(matches!(
        decode_sidecar(&bytes, binding()),
        Err(SidecarError::InvalidEncoding)
    ));
}

#[test]
fn bridged_observation_enumerates_fabric_quins_with_plane_separation_and_5_field_parity() {
    use etalon_sense_core::{
        capability::Modality,
        quantity::{SpectralAxisKind, SpectralBand},
        spectrum::{AcousticBand, SpectralCoverage},
    };
    use qualia_core_db::net::qdnf::{
        authority::Plane,
        errors::QdnfError,
        fabric::{attempt_join, JoinAttempt, JoinKey},
    };
    use qualia_core_db::{q_hash, NQuin};

    let rf_band = SpectralBand::new(SpectralAxisKind::CarrierHertz, 88.0e6, 108.0e6).unwrap();
    let rf_cov =
        SpectralCoverage::new(Modality::Sdr, Some(rf_band), None, None, None, None).unwrap();
    let bridged_rf = bridge_observation(&Authority, 21, 17, observation(), binding())
        .unwrap()
        .with_spectral_coverage(rf_cov);

    let zero_quin = NQuin {
        subject: 0,
        predicate: 0,
        object: 0,
        context: 0,
        metadata: 0,
        parity: 0,
    };
    let mut rf_quins = [zero_quin; 8];
    let count = bridged_rf.enumerate_fabric_quins(&mut rf_quins).unwrap();
    assert_eq!(count, 8);
    for (i, q) in rf_quins.iter().enumerate() {
        assert!(
            q.verify_ecc_parity(),
            "fabric quin {i} must satisfy canonical 5-field XOR parity"
        );
        assert_ne!(
            q.context,
            q_hash("urn:qualia:idf:plane:Entity"),
            "sensor observation quin {i} must never live in the NaturalAgent / Entity plane"
        );
    }
    assert_eq!(rf_quins[5].context, q_hash("urn:qualia:idf:plane:Instrument"));
    assert_eq!(rf_quins[5].object, q_hash("urn:qualia:idf:instrument:SensorId"));
    assert_eq!(rf_quins[6].context, q_hash("urn:qualia:idf:plane:Handle"));
    assert_eq!(
        rf_quins[6].object,
        q_hash("urn:qualia:idf:shape:EnvironmentPredicateShape")
    );
    assert_eq!(
        rf_quins[7].object,
        q_hash("urn:qualia:etalon:spectrum:Electromagnetic")
    );
    assert_ne!(rf_quins[7].metadata & (1u64 << 63), 0, "EMF flag bit must be set");

    // Mechanical AcousticBand must never set the EMF bit and must remain distinct from the 9 EMF modalities.
    let acoustic_band = AcousticBand::new(20.0, 20_000.0).unwrap();
    let acoustic_cov = SpectralCoverage::new(
        Modality::Microphone,
        None,
        None,
        None,
        None,
        Some(acoustic_band),
    )
    .unwrap();
    let bridged_acoustic = bridge_observation(&Authority, 22, 17, observation(), binding())
        .unwrap()
        .with_spectral_coverage(acoustic_cov);
    let mut acoustic_quins = [zero_quin; 8];
    assert_eq!(
        bridged_acoustic
            .enumerate_fabric_quins(&mut acoustic_quins)
            .unwrap(),
        8
    );
    for q in &acoustic_quins {
        assert!(q.verify_ecc_parity());
    }
    assert_eq!(
        acoustic_quins[7].object,
        q_hash("urn:qualia:etalon:spectrum:AcousticBand")
    );
    assert_eq!(
        acoustic_quins[7].metadata & (1u64 << 63),
        0,
        "Mechanical AcousticBand must not set the EMF bit"
    );

    // Sensor evidence (BLE MAC / SDR / NFC) cannot join directly to a NaturalAgent.
    assert_eq!(
        attempt_join(JoinAttempt {
            left: Plane::Instrument,
            right: Plane::Entity,
            key: JoinKey::SensorCoPresence,
        }),
        Err(QdnfError::Denied)
    );
}

#[test]
fn process_permit_never_implies_store_share_or_export_without_separate_permit() {
    use qualia_core_db::sensing::sidecar_attestation::SIDECAR_RECEIPT_BYTES;
    let grant = signed_grant();
    let process_permit = permit(&grant, SensingAction::Process, 17);
    let bridged = bridge_observation_with_consent(
        &ConsentAuthority(true),
        process_permit,
        &grant,
        &Identity,
        &SensorScope(true),
        &Revocations(false),
        &mut Clock(60),
        21,
        17,
        observation(),
        binding(),
    )
    .expect("Process permit authorizes bridge_observation_with_consent");

    // Neither Store, Share, nor Export permits may be substituted for Process, and a Process
    // permit may never be used to prepare a Store receipt.
    for non_process in [SensingAction::Store, SensingAction::Share, SensingAction::Export] {
        assert!(matches!(
            bridge_observation_with_consent(
                &ConsentAuthority(true),
                permit(&grant, non_process, 17),
                &grant,
                &Identity,
                &SensorScope(true),
                &Revocations(false),
                &mut Clock(60),
                21,
                17,
                observation(),
                binding(),
            ),
            Err(BridgeError::PermitScopeMismatch)
        ));
    }

    let key = SigningKey::from_bytes(&[11; 32]);
    let mut sidecar = [0xa5; SIDECAR_SIZE];
    let mut receipt = [0xa5; SIDECAR_RECEIPT_BYTES];
    for non_store in [SensingAction::Process, SensingAction::Share, SensingAction::Export] {
        let res = prepare_store_receipt(
            &StoreAuthority(true),
            permit(&grant, non_store, 17),
            &grant,
            &Identity,
            &SensorScope(true),
            &Revocations(false),
            &mut Clock(60),
            &bridged,
            &[5; 16],
            16,
            [1; 32],
            0,
            None,
            &key,
            &mut sidecar,
            &mut receipt,
        );
        assert!(matches!(res, Err(StoreReceiptError::ScopeMismatch)));
    }
}
