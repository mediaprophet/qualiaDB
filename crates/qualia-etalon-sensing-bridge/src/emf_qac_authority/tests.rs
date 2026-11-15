use ed25519_dalek::SigningKey;
use etalon_sense_core::{
    evidence::EvidenceOrigin,
    observation::{
        HostPolicyDecision, MeasurementUncertainty, ObservationEnvelope, ObservationKind,
        ObservationTimeAnchor, QuantityUnitIds,
    },
    quality::{
        CalibrationArtifactId, CalibrationRecord, CalibrationState, QualityRecord,
        SourceDescriptor, SourceKind,
    },
    time::ClockStamp,
};
use qualia_core_db::{
    container_10d as c,
    governance::consent_contract::{ConsentGrant, ConsentScope},
    sensing::{
        acquisition::{Clock, Original, Rate, Timing},
        authorization::{
            authorize_sensing, AuthorizationError, IdentityAuthority, RevocationAuthority,
            SensingAction, SensingModality, SensingPermit, SensingRequest, SensingScopeAuthority,
            TrustedUnixClock,
        },
        record::ResolvedFieldPayloadBinding,
        MeasurementModality, PhysicalFieldSemantics, SemanticId,
    },
};

use crate::{
    emf_qac_authority::{
        calibration_clock_bridge::{
            apply_single_calibration_pair, convert_iq_between_emf_and_qac,
            reject_unconjugated_iq_memcpy, verify_phase_coherent_envelope_join,
            verify_same_clock_domain, BridgeCalibrationStage, BridgeCalibrationWindow,
            BridgeIqConvention, BridgeIqSample, BridgeRationalInstant, CalibrationClockBridgeError,
        },
        emf_iq_dose_bio::{
            bridge_emf_mode0105_iq_with_consent, bridge_emf_mode0106_ionizing_with_consent,
            bridge_emf_mode0107_bio_dipole_with_consent, BridgeEpistemicClass,
            BridgeUncertaintyKind, ComponentTargetedUncertainty, EmfAuthorityBridgeError,
            EmfBridgeReceiptReplayWindow, EmfMode0105IqInput, EmfMode0106IonizingInput,
            EmfMode0107BioDipoleInput, IonizingObservationKind, SignedEmfBridgeReceipt,
            EMF_MODE_0105_CALIBRATED_IQ_STFT, EMF_MODE_0106_IONIZING_RADIATION,
            EMF_MODE_0107_BIOELECTRIC_DIPOLE,
        },
    },
    BridgeAuthority, ConsentBridgeAuthority,
};

pub(super) fn id(n: u64) -> SemanticId {
    SemanticId::new(n).unwrap()
}

pub(super) fn test_binding(data: &[u8], modality: MeasurementModality) -> ResolvedFieldPayloadBinding {
    let section = c::SectionInput {
        section_type: c::SectionType::FieldSidecar,
        alignment_tier: c::AlignmentTier::Word,
        stride: 0,
        element_count: 0,
        payload: data,
    };
    let typed = c::TypedFieldRecord {
        field_id: 9,
        kind: c::FIELD_KIND_SCALAR,
        flags: 0,
        unit_id: 11,
        payload_section_id: c::SectionType::FieldSidecar as u32,
        payload_offset: 0,
        payload_length: data.len() as u64,
    };
    let identity = c::ManifoldIdentityV2 {
        entity_id: c::StableEntityId([7; 16]),
        profile_id: 1,
        domain_id: 2,
        coordinate_convention_id: 3,
        coordinates: [0.0; 10],
        external_digest: None,
    };
    let mut manifest = [0; c::MANIFOLD_IDENTITY_V2_MAX_BYTES];
    let mut sections = [section; 2];
    let mut bytes = [0; 4096];
    let len = c::encode_container_with_manifold_identity(
        &c::Container10dHeader::proposed(),
        &[section],
        &identity,
        &[typed],
        &mut manifest,
        &mut sections,
        &mut bytes,
    )
    .unwrap();
    c::seal_whole_file_crc32c(&mut bytes[..len]);
    let mut scratch = [c::TypedFieldRecord::default(); 1];
    let mut fields = [c::TypedFieldRecord::default(); 1];
    let resolver =
        c::typed_payloads::TypedPayloadResolver::new(&bytes[..len], &[], &mut scratch, &mut fields)
            .unwrap();
    let field = c::field_section::FieldDescriptor {
        name_hash: 22,
        unit_hash: 33,
        type_kind: c::field_section::FieldTypeKind::Spectral as u8,
        support: c::field_section::FieldSupportKind::Stream as u8,
        representation: c::field_section::FieldRepresentationKind::Sampled as u8,
        reserved: 0,
        reserved2: 0,
        sample_offset: 0,
    };
    let physical =
        PhysicalFieldSemantics::from_existing(&field, &typed, modality, id(13)).unwrap();
    ResolvedFieldPayloadBinding::new(physical, &resolver, [1; 16]).unwrap()
}

pub(super) fn calibrated_observation(
    clock_id: u64,
    restart_epoch: u32,
    ticks_ns: u64,
    uncertainty_ns: u64,
    cal_id: [u8; 32],
) -> ObservationEnvelope {
    let artifact_id = CalibrationArtifactId::new(cal_id).unwrap();
    ObservationEnvelope::new(
        ObservationKind::RecordedFrame,
        SourceDescriptor::new(41, SourceKind::Recorded).unwrap(),
        Some(8),
        Some(2),
        Some(9),
        Some(ClockStamp::new(clock_id, restart_epoch, ticks_ns, uncertainty_ns).unwrap()),
        QuantityUnitIds::new(Some(301), Some(302)).unwrap(),
        MeasurementUncertainty::Unknown,
        QualityRecord::Unknown,
        CalibrationRecord::new(CalibrationState::Calibrated { artifact_id }).unwrap(),
        EvidenceOrigin::Unknown,
        None,
        HostPolicyDecision::NotEvaluated,
    )
    .unwrap()
    .with_time_anchor(ObservationTimeAnchor::FirstSample)
    .unwrap()
}

pub(super) struct TestHostAuthority {
    pub allow: bool,
}

impl BridgeAuthority for TestHostAuthority {
    fn confirm_mapping(
        &self,
        observation: &ObservationEnvelope,
        qualia_source_id: u64,
        physical: &PhysicalFieldSemantics,
    ) -> bool {
        self.allow
            && observation.source().source_id() == 41
            && qualia_source_id == 17
            && physical.physical_quantity() == id(13)
    }

    fn approve(
        &self,
        _observation: &ObservationEnvelope,
        qualia_source_id: u64,
        _binding: &ResolvedFieldPayloadBinding,
    ) -> bool {
        self.allow && qualia_source_id == 17
    }

    fn confirm_clock_namespace(&self, _observation: &ObservationEnvelope) -> bool {
        self.allow
    }

    fn confirm_calibration_namespace(&self, _observation: &ObservationEnvelope) -> bool {
        self.allow
    }
}

impl ConsentBridgeAuthority for TestHostAuthority {
    fn confirm_consent_scope(
        &self,
        _grant: &ConsentGrant,
        request: &SensingRequest,
        _observation: &ObservationEnvelope,
        _binding: &ResolvedFieldPayloadBinding,
    ) -> bool {
        self.allow && request.source_id() == 17 && request.action() == SensingAction::Process
    }
}

pub(super) struct TestPolicyAuthorities {
    pub revoked: bool,
    pub now_unix: u64,
}

impl IdentityAuthority for TestPolicyAuthorities {
    fn verify_parties(
        &self,
        grant: &ConsentGrant,
        actor_id: &[u8; 32],
    ) -> Result<(), AuthorizationError> {
        if &grant.recipient_did == actor_id {
            Ok(())
        } else {
            Err(AuthorizationError::ActorMismatch)
        }
    }
}

impl SensingScopeAuthority for TestPolicyAuthorities {
    fn verify_scope(
        &self,
        _grant: &ConsentGrant,
        _grant_digest: &[u8; 32],
        request: &SensingRequest,
    ) -> Result<(), AuthorizationError> {
        if request.source_id() == 17 {
            Ok(())
        } else {
            Err(AuthorizationError::UnsupportedSensorScope)
        }
    }
}

impl RevocationAuthority for TestPolicyAuthorities {
    fn is_revoked(
        &self,
        _grant_id: &[u8; 32],
        _grant_digest: &[u8; 32],
        _current_time_unix: u64,
    ) -> Result<bool, AuthorizationError> {
        Ok(self.revoked)
    }
}

#[derive(Clone, Copy)]
pub(super) struct TestUnixClock {
    pub now_unix: u64,
}

impl TrustedUnixClock for TestUnixClock {
    fn now_unix(&mut self) -> Result<u64, AuthorizationError> {
        Ok(self.now_unix)
    }
}

impl TestPolicyAuthorities {
    pub(super) fn clock(&self) -> TestUnixClock {
        TestUnixClock {
            now_unix: self.now_unix,
        }
    }
}

pub(super) fn issue_grant_and_permit(
    modality: SensingModality,
    action: SensingAction,
    source_id: u64,
    purpose_hash: u64,
) -> (ConsentGrant, SensingPermit, [u8; 32], TestPolicyAuthorities) {
    let principal_key = SigningKey::from_bytes(&[11u8; 32]);
    let actor_id = [22u8; 32];
    let grant = ConsentGrant::new_signed(
        [33u8; 32],
        &principal_key,
        actor_id,
        ConsentScope(ConsentScope::VITALS),
        purpose_hash,
        1_700_000_000,
        1_700_010_000,
        1,
    );
    let request = SensingRequest::new(actor_id, source_id, modality, action, purpose_hash).unwrap();
    let policy = TestPolicyAuthorities {
        revoked: false,
        now_unix: 1_700_000_500,
    };
    let mut clock = policy.clock();
    let permit = authorize_sensing(&grant, request, &policy, &policy, &policy, &mut clock).unwrap();
    (grant, permit, actor_id, policy)
}

#[test]
fn half_open_calibration_window_and_clock_discipline_enforce_exact_boundaries() {
    let timing = Timing {
        scale: 0,
        epoch_seconds: 0,
        epoch_nanoseconds: 0,
        ticks: Rate::new(1_000, 1).unwrap(),
        samples: Rate::new(48_000, 1).unwrap(),
        start_tick: 0,
    };
    let clock_a = Clock::new(id(2), 3, timing).unwrap();
    let clock_b = Clock::new(id(99), 3, timing).unwrap();

    // Cross-origin relative clocks with identical rate/epoch/start_tick must be rejected.
    assert_eq!(
        verify_same_clock_domain(clock_a, clock_b),
        Err(CalibrationClockBridgeError::CrossClockOriginRejected)
    );

    let valid_from = BridgeRationalInstant::from_clock_tick_subtick(clock_a, 100, 0, 1).unwrap();
    let valid_until = BridgeRationalInstant::from_clock_tick_subtick(clock_a, 200, 0, 1).unwrap();
    let window = BridgeCalibrationWindow::new(valid_from, valid_until).unwrap();

    // 1) valid_from - 1 tick -> NotYetValid
    let before = BridgeRationalInstant::from_clock_tick_subtick(clock_a, 99, 0, 1).unwrap();
    assert_eq!(
        window.verify_contains(before),
        Err(CalibrationClockBridgeError::NotYetValid)
    );
    // 2) valid_from -> Ok
    assert_eq!(window.verify_contains(valid_from), Ok(()));
    // 3) valid_until - 1 subtick (tick 199 + 999/1000) -> Ok
    let just_before_until =
        BridgeRationalInstant::from_clock_tick_subtick(clock_a, 199, 999, 1000).unwrap();
    assert_eq!(window.verify_contains(just_before_until), Ok(()));
    // 4) exact valid_until -> ExpiredAtOrAfterUntil (zero tolerance)
    assert_eq!(
        window.verify_contains(valid_until),
        Err(CalibrationClockBridgeError::ExpiredAtOrAfterUntil)
    );
}

#[test]
fn phase_uncertainty_and_carrier_convention_guard_reject_incoherence_and_memcpy() {
    let cal_id = [0xAA; 32];
    // 5 ns uncertainty at 10 MHz -> delta_phi = 2 * pi * 1e7 * 5e-9 = 0.1 * pi ~= 0.314159 rad
    let left = calibrated_observation(2, 3, 150_000_000, 5, cal_id);
    let right = calibrated_observation(2, 3, 150_000_000, 5, cal_id);

    let total_phi =
        verify_phase_coherent_envelope_join(&left, &right, 10.0e6, 0.70).unwrap();
    assert!(total_phi > 0.62 && total_phi < 0.63);
    assert_eq!(
        verify_phase_coherent_envelope_join(&left, &right, 10.0e6, 0.50),
        Err(CalibrationClockBridgeError::PhaseUncertaintyExceedsBudget)
    );

    // Raw memcpy between EMF negative carrier and QAC positive carrier is rejected.
    assert_eq!(
        reject_unconjugated_iq_memcpy(
            BridgeIqConvention::EmfNegativeCarrier,
            BridgeIqConvention::QacPositiveCarrier
        ),
        Err(CalibrationClockBridgeError::UnconjugatedCarrierMemcpyRejected)
    );

    // Explicit conjugation z_QAC = conj(z_EMF) preserves the physical real waveform at any t.
    let emf_sample = BridgeIqSample {
        i: 1.25,
        q: -0.75,
        convention: BridgeIqConvention::EmfNegativeCarrier,
        carrier_hz: 5.0e6,
        reference_phase_rad: 0.3,
        signed_baseband_offset_hz: 1200.0,
    };
    let qac_sample =
        convert_iq_between_emf_and_qac(emf_sample, BridgeIqConvention::QacPositiveCarrier, 0.0)
            .unwrap();
    let real_emf = emf_sample.reconstruct_real_at(1.7e-7).unwrap();
    let real_qac = qac_sample.reconstruct_real_at(1.7e-7).unwrap();
    assert!((real_emf - real_qac).abs() < 1e-12);
    assert_eq!(qac_sample.signed_baseband_offset_hz, -1200.0);

    // Single-application calibration: (3, 4) * 2 -> (6, 8) once; second application rejected.
    let (cal_pair, stage, units_per_sample, offset) = apply_single_calibration_pair(
        BridgeCalibrationStage::RawCounts,
        (3.0, 4.0),
        2.0,
        0.0,
        true,
    )
    .unwrap();
    assert_eq!(cal_pair, (6.0, 8.0));
    assert_eq!(stage, BridgeCalibrationStage::PhysicalBaseUnits);
    assert_eq!((units_per_sample, offset), (1.0, 0.0));
    assert_eq!(
        apply_single_calibration_pair(stage, cal_pair, 2.0, 0.0, true),
        Err(CalibrationClockBridgeError::DoubleCalibrationRejected)
    );
}

#[test]
fn emf_modes_0105_0106_0107_bridge_positive_and_reject_tamper_replay_and_invalid_physics() {
    let raw_bytes = [0x42u8; 16];
    let payload_digest = Original::admit(&raw_bytes, 16).unwrap().digest();
    let cal_id = [0x55u8; 32];
    let obs = calibrated_observation(2, 3, 150_000_000, 1, cal_id);
    let valid_from = BridgeRationalInstant::new(id(2), 3, 0, 100_000_000, 1_000_000_000).unwrap();
    let valid_until = BridgeRationalInstant::new(id(2), 3, 0, 200_000_000, 1_000_000_000).unwrap();
    let cal_window = BridgeCalibrationWindow::new(valid_from, valid_until).unwrap();
    let receipt_signer = SigningKey::from_bytes(&[77u8; 32]);
    let host = TestHostAuthority { allow: true };

    // --- Mode 0x0105 positive + replay rejection ---
    let (grant, permit, actor_id, policy) =
        issue_grant_and_permit(SensingModality::Sdr, SensingAction::Process, 17, 9001);
    let receipt_0105 = SignedEmfBridgeReceipt::sign(
        &receipt_signer,
        [1u8; 32],
        EMF_MODE_0105_CALIBRATED_IQ_STFT,
        17,
        actor_id,
        9001,
        SensingAction::Process,
        payload_digest,
        cal_id,
        1_700_000_000,
        1_700_010_000,
    )
    .unwrap();
    let digest_0105 = receipt_0105.receipt_digest();
    let mut replay_window = EmfBridgeReceiptReplayWindow::<8>::new();
    let input_0105 = EmfMode0105IqInput {
        record_id: 501,
        qualia_source_id: 17,
        observation: obs,
        binding: test_binding(&raw_bytes, MeasurementModality::Electromagnetic),
        payload_digest,
        calibration_artifact_id: cal_id,
        calibration_window: cal_window,
        calibration_stage: BridgeCalibrationStage::RawCounts,
        scale_factor: 2.0,
        offset: 0.0,
        raw_iq_sample: (3.0, 4.0),
        carrier_hz: 100.0e6,
        baseband_sample_rate: Rate::new(2_000_000, 1).unwrap(),
        lo_phase_rad: 0.25,
        iq_convention: BridgeIqConvention::EmfNegativeCarrier,
        max_phase_uncertainty_rad: 1.0,
        receipt: receipt_0105,
        expected_receipt_digest: digest_0105,
    };
    let bridged_0105 = bridge_emf_mode0105_iq_with_consent(
        &host,
        permit,
        &grant,
        &policy,
        &policy,
        &policy,
        &mut policy.clock(),
        &mut replay_window,
        input_0105,
    )
    .unwrap();
    assert_eq!(bridged_0105.calibrated_iq_v_per_m, (6.0, 8.0));
    assert_eq!(*bridged_0105.bridged.observation(), obs);

    let (_, permit_replay, _, policy_replay) =
        issue_grant_and_permit(SensingModality::Sdr, SensingAction::Process, 17, 9001);
    let mut input_replay = input_0105;
    input_replay.binding = test_binding(&raw_bytes, MeasurementModality::Electromagnetic);
    assert!(matches!(
        bridge_emf_mode0105_iq_with_consent(
            &host, permit_replay, &grant, &policy_replay, &policy_replay,
            &policy_replay, &mut policy_replay.clock(), &mut replay_window, input_replay,
        ),
        Err(EmfAuthorityBridgeError::ReplayedAuthorityReceipt)
    ));

    // --- Mode 0x0106: reject inferring dose from attenuation & accept valid measured dose ---
    let (grant_6, permit_6, _, policy_6) =
        issue_grant_and_permit(SensingModality::FieldInstrument, SensingAction::Process, 17, 9002);
    let receipt_0106 = SignedEmfBridgeReceipt::sign(
        &receipt_signer, [2u8; 32], EMF_MODE_0106_IONIZING_RADIATION, 17, actor_id,
        9002, SensingAction::Process, payload_digest, cal_id, 1_700_000_000, 1_700_010_000,
    ).unwrap();
    let digest_0106 = receipt_0106.receipt_digest();
    let dose_unc = ComponentTargetedUncertainty {
        target_quantity: id(701),
        target_unit: id(702),
        kind: BridgeUncertaintyKind::StandardAbsolute,
        value: 0.001,
    };
    let mut input_0106 = EmfMode0106IonizingInput {
        record_id: 502,
        qualia_source_id: 17,
        observation: obs,
        binding: test_binding(&raw_bytes, MeasurementModality::Electromagnetic),
        payload_digest,
        calibration_artifact_id: cal_id,
        calibration_window: cal_window,
        epistemic_class: BridgeEpistemicClass::Observation,
        unlabeled_scalar_uncertainty_present: false,
        kind: IonizingObservationKind::MeasuredDose {
            absorbed_dose_gy: 0.025,
            absorbed_dose_uncertainty: dose_unc,
            equivalent_dose_sv: None,
            equivalent_dose_uncertainty: None,
            weighting_model_id: None,
            infer_dose_from_attenuation_attempted: true,
        },
        receipt: receipt_0106,
        expected_receipt_digest: digest_0106,
    };
    assert!(matches!(
        bridge_emf_mode0106_ionizing_with_consent(
            &host, permit_6, &grant_6, &policy_6, &policy_6,
            &policy_6, &mut policy_6.clock(), &mut replay_window, input_0106,
        ),
        Err(EmfAuthorityBridgeError::DoseInferredFromAttenuationRejected)
    ));
    let (_, permit_6_ok, _, policy_6_ok) =
        issue_grant_and_permit(SensingModality::FieldInstrument, SensingAction::Process, 17, 9002);
    input_0106.binding = test_binding(&raw_bytes, MeasurementModality::Electromagnetic);
    input_0106.kind = IonizingObservationKind::MeasuredDose {
        absorbed_dose_gy: 0.025,
        absorbed_dose_uncertainty: dose_unc,
        equivalent_dose_sv: None,
        equivalent_dose_uncertainty: None,
        weighting_model_id: None,
        infer_dose_from_attenuation_attempted: false,
    };
    assert!(bridge_emf_mode0106_ionizing_with_consent(
        &host, permit_6_ok, &grant_6, &policy_6_ok, &policy_6_ok,
        &policy_6_ok, &mut policy_6_ok.clock(), &mut replay_window, input_0106,
    ).is_ok());

    // --- Mode 0x0107: reject singular dipole r == r0, accept non-singular observation ---
    let (grant_7, permit_7, _, policy_7) =
        issue_grant_and_permit(SensingModality::FieldInstrument, SensingAction::Process, 17, 9003);
    let receipt_0107 = SignedEmfBridgeReceipt::sign(
        &receipt_signer, [3u8; 32], EMF_MODE_0107_BIOELECTRIC_DIPOLE, 17, actor_id,
        9003, SensingAction::Process, payload_digest, cal_id, 1_700_000_000, 1_700_010_000,
    ).unwrap();
    let digest_0107 = receipt_0107.receipt_digest();
    let mut input_0107 = EmfMode0107BioDipoleInput {
        record_id: 503,
        qualia_source_id: 17,
        observation: obs,
        binding: test_binding(&raw_bytes, MeasurementModality::Electromagnetic),
        payload_digest,
        calibration_artifact_id: cal_id,
        calibration_window: cal_window,
        anatomy_ontology_id: id(801),
        anatomy_identifier: id(802),
        consent_receipt_digest: digest_0107,
        source_dipole_position_m: [0.01, 0.02, 0.03],
        observation_electrode_position_m: Some([0.01, 0.02, 0.03]),
        reference_potential_id: Some(id(803)),
        dipole_moment_amp_m: [1.0e-9, 0.0, 0.0],
        extracellular_potential_v: Some(1.5e-4),
        dipole_moment_uncertainty: dose_unc,
        potential_uncertainty: Some(dose_unc),
        receipt: receipt_0107,
        expected_receipt_digest: digest_0107,
    };
    assert!(matches!(
        bridge_emf_mode0107_bio_dipole_with_consent(
            &host, permit_7, &grant_7, &policy_7, &policy_7,
            &policy_7, &mut policy_7.clock(), &mut replay_window, input_0107,
        ),
        Err(EmfAuthorityBridgeError::SingularDipoleSourceEvaluationRejected)
    ));
    let (_, permit_7_ok, _, policy_7_ok) =
        issue_grant_and_permit(SensingModality::FieldInstrument, SensingAction::Process, 17, 9003);
    input_0107.binding = test_binding(&raw_bytes, MeasurementModality::Electromagnetic);
    input_0107.observation_electrode_position_m = Some([0.04, 0.02, 0.03]);
    let bridged_0107 = bridge_emf_mode0107_bio_dipole_with_consent(
        &host, permit_7_ok, &grant_7, &policy_7_ok, &policy_7_ok,
        &policy_7_ok, &mut policy_7_ok.clock(), &mut replay_window, input_0107,
    ).unwrap();
    assert!((bridged_0107.source_to_electrode_distance_m.unwrap() - 0.03).abs() < 1e-12);
}
