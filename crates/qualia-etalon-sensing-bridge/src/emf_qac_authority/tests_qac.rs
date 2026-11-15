use ed25519_dalek::SigningKey;
use qualia_core_db::sensing::{
    acquisition::{Original, Rate},
    authorization::{SensingAction, SensingModality},
    MeasurementModality,
};

use super::tests::{
    calibrated_observation, id, issue_grant_and_permit, test_binding, TestHostAuthority,
};
use crate::emf_qac_authority::{
    calibration_clock_bridge::{
        BridgeCalibrationStage, BridgeCalibrationWindow, BridgeIqConvention,
        BridgeRationalInstant, CalibrationClockBridgeError,
    },
    emf_iq_dose_bio::BridgeUncertaintyKind,
    emf_iq_dose_bio::ComponentTargetedUncertainty,
    qac_haptic_ultrasound::{
        admit_qac_simulator_command, bridge_qac_mode0106_haptic_with_consent,
        bridge_qac_mode0107_ultrasound_with_consent, reject_passive_artifact_for_actuation,
        BridgeProvenanceRole, HapticForceConvention, PassiveObservationArtifactKind,
        QacAuthorityBridgeError, QacCommandScope, QacHapticRole, QacMode0106HapticInput,
        QacMode0107UltrasoundInput, QacReceiptReplayLedger, QacSimulatorCommandEnvelope,
        QacUltrasoundRole, SignedQacCommandReceipt, SignedQacObservationReceipt,
        ThermalIndexVariant, QAC_MODE_0106_HAPTIC_MECHANICAL, QAC_MODE_0107_ULTRASOUND_BEAM,
    },
};

#[test]
fn qac_modes_0106_0107_and_simulator_actuation_gate_enforce_role_and_expiry_separation() {
    let raw_bytes = [0x24u8; 16];
    let payload_digest = Original::admit(&raw_bytes, 16).unwrap().digest();
    let cal_id = [0x66u8; 32];
    let obs = calibrated_observation(2, 3, 150_000_000, 1, cal_id);
    let valid_from = BridgeRationalInstant::new(id(2), 3, 0, 100_000_000, 1_000_000_000).unwrap();
    let valid_until = BridgeRationalInstant::new(id(2), 3, 0, 200_000_000, 1_000_000_000).unwrap();
    let cal_window = BridgeCalibrationWindow::new(valid_from, valid_until).unwrap();
    let receipt_signer = SigningKey::from_bytes(&[88u8; 32]);
    let host = TestHostAuthority { allow: true };
    let mut qac_ledger = QacReceiptReplayLedger::<8>::new();

    let (grant, permit, actor_id, policy) =
        issue_grant_and_permit(SensingModality::Microphone, SensingAction::Process, 17, 9101);
    let receipt_haptic = SignedQacObservationReceipt::sign(
        &receipt_signer,
        [4u8; 32],
        QAC_MODE_0106_HAPTIC_MECHANICAL,
        SensingModality::Microphone,
        17,
        actor_id,
        9101,
        SensingAction::Process,
        payload_digest,
        cal_id,
        1_700_000_000,
        1_700_010_000,
    )
    .unwrap();
    let digest_haptic = receipt_haptic.receipt_digest();
    let unc = ComponentTargetedUncertainty {
        target_quantity: id(901),
        target_unit: id(902),
        kind: BridgeUncertaintyKind::StandardAbsolute,
        value: 0.01,
    };
    let mut haptic_in = QacMode0106HapticInput {
        record_id: 601,
        qualia_source_id: 17,
        observation: obs,
        binding: test_binding(&raw_bytes, MeasurementModality::Acoustic),
        payload_digest,
        calibration_artifact_id: cal_id,
        calibration_window: cal_window,
        role: QacHapticRole::ActuatorCommand,
        contact_frame_id: 1,
        force_convention: HapticForceConvention::ForceOnBody,
        force_vector_n: [0.0, 0.0, 2.5],
        displacement_vector_m: [0.0, 0.0, 0.001],
        contact_area_m2: 1.0e-4,
        force_uncertainty: unc,
        displacement_uncertainty: unc,
        receptor_model_id: None,
        receipt: receipt_haptic,
        expected_receipt_digest: digest_haptic,
    };
    // Command role in observation bridge is rejected.
    assert!(matches!(
        bridge_qac_mode0106_haptic_with_consent(
            &host,
            permit,
            &grant,
            &policy,
            &policy,
            &policy,
            &mut policy.clock(),
            &mut qac_ledger,
            haptic_in,
        ),
        Err(QacAuthorityBridgeError::CommandRoleRejectedInObservationBridge)
    ));
    let (_, permit_ok, _, policy_ok) =
        issue_grant_and_permit(SensingModality::Microphone, SensingAction::Process, 17, 9101);
    haptic_in.binding = test_binding(&raw_bytes, MeasurementModality::Acoustic);
    haptic_in.role = QacHapticRole::Measurement;
    assert!(bridge_qac_mode0106_haptic_with_consent(
        &host,
        permit_ok,
        &grant,
        &policy_ok,
        &policy_ok,
        &policy_ok,
        &mut policy_ok.clock(),
        &mut qac_ledger,
        haptic_in,
    )
    .is_ok());

    // Ultrasound Mode 0x0107 positive + missing TI model rejection.
    let (grant_u, permit_u, _, policy_u) =
        issue_grant_and_permit(SensingModality::Microphone, SensingAction::Process, 17, 9102);
    let receipt_us = SignedQacObservationReceipt::sign(
        &receipt_signer,
        [5u8; 32],
        QAC_MODE_0107_ULTRASOUND_BEAM,
        SensingModality::Microphone,
        17,
        actor_id,
        9102,
        SensingAction::Process,
        payload_digest,
        cal_id,
        1_700_000_000,
        1_700_010_000,
    )
    .unwrap();
    let digest_us = receipt_us.receipt_digest();
    let mut us_in = QacMode0107UltrasoundInput {
        record_id: 602,
        qualia_source_id: 17,
        observation: obs,
        binding: test_binding(&raw_bytes, MeasurementModality::Acoustic),
        payload_digest,
        calibration_artifact_id: cal_id,
        calibration_window: cal_window,
        role: QacUltrasoundRole::BasebandIq,
        array_frame_id: 2,
        center_frequency_hz: 5.0e6,
        rf_sample_rate: Rate::new(40_000_000, 1).unwrap(),
        prf_rate: Rate::new(4_000, 1).unwrap(),
        iq_convention: BridgeIqConvention::QacPositiveCarrier,
        calibration_stage: BridgeCalibrationStage::RawCounts,
        scale_factor: 2.0,
        offset: 0.0,
        raw_iq_or_rf_pair: (3.0, 4.0),
        max_phase_uncertainty_rad: 0.5,
        derated_peak_rarefactional_pressure_mpa: 1.2,
        mechanical_index: 0.53,
        thermal_index: Some(0.4),
        thermal_index_variant: Some(ThermalIndexVariant::Tis),
        thermal_model_id: None,
        receipt: receipt_us,
        expected_receipt_digest: digest_us,
    };
    assert!(matches!(
        bridge_qac_mode0107_ultrasound_with_consent(
            &host,
            permit_u,
            &grant_u,
            &policy_u,
            &policy_u,
            &policy_u,
            &mut policy_u.clock(),
            &mut qac_ledger,
            us_in,
        ),
        Err(QacAuthorityBridgeError::MissingReceptorOrThermalModel)
    ));
    let (_, permit_u_ok, _, policy_u_ok) =
        issue_grant_and_permit(SensingModality::Microphone, SensingAction::Process, 17, 9102);
    us_in.binding = test_binding(&raw_bytes, MeasurementModality::Acoustic);
    us_in.thermal_model_id = Some(id(950));
    let bridged_us = bridge_qac_mode0107_ultrasound_with_consent(
        &host,
        permit_u_ok,
        &grant_u,
        &policy_u_ok,
        &policy_u_ok,
        &policy_u_ok,
        &mut policy_u_ok.clock(),
        &mut qac_ledger,
        us_in,
    )
    .unwrap();
    assert_eq!(bridged_us.calibrated_sample_pair, (6.0, 8.0));

    // Passive artifacts can never authorize simulator actuation.
    for artifact in [
        PassiveObservationArtifactKind::StoredIonizingDoseGyOrSv,
        PassiveObservationArtifactKind::StoredUltrasoundMechanicalIndex,
        PassiveObservationArtifactKind::StoredUltrasoundThermalIndex,
        PassiveObservationArtifactKind::StoredHapticContactForceN,
        PassiveObservationArtifactKind::StoredPitchOrVoiceFeature,
        PassiveObservationArtifactKind::StoredContainerCrcOrBlake3Digest,
        PassiveObservationArtifactKind::StoredCalibrationCertificateDigest,
        PassiveObservationArtifactKind::PassiveObservationConsentId,
    ] {
        assert_eq!(
            reject_passive_artifact_for_actuation(artifact),
            Err(QacAuthorityBridgeError::PassiveObservationCannotAuthorizeActuation)
        );
    }

    // Simulator command admission: positive inside [from, until), rejected at exact until, replay, or revocation.
    let cmd_receipt = SignedQacCommandReceipt::sign(
        &receipt_signer,
        [6u8; 32],
        QacCommandScope::UltrasoundTransmitCommandSchema208,
        17,
        actor_id,
        9102,
        [0xAA; 32],
        2,
        [0xBB; 32],
        1_700_000_000,
        1_700_010_000,
    )
    .unwrap();
    let cmd_digest = cmd_receipt.receipt_digest();
    let mut cmd_env = QacSimulatorCommandEnvelope {
        scope: QacCommandScope::UltrasoundTransmitCommandSchema208,
        provenance_role: BridgeProvenanceRole::Command,
        device_profile_digest: [0xAA; 32],
        contact_or_array_frame_id: 2,
        command_payload_digest: [0xBB; 32],
        stored_authority_receipt_digest: cmd_digest,
        execution_instant: BridgeRationalInstant::new(id(2), 3, 0, 150_000_000, 1_000_000_000)
            .unwrap(),
        command_validity_window: cal_window,
        receipt: cmd_receipt,
    };
    let verified_cmd = admit_qac_simulator_command(
        &cmd_env,
        &grant_u,
        &policy_u_ok,
        &mut policy_u_ok.clock(),
        &mut qac_ledger,
    )
    .unwrap();
    assert_eq!(verified_cmd.authority_receipt_digest, cmd_digest);

    // Replay of the same command receipt is rejected.
    assert_eq!(
        admit_qac_simulator_command(
            &cmd_env,
            &grant_u,
            &policy_u_ok,
            &mut policy_u_ok.clock(),
            &mut qac_ledger
        ),
        Err(QacAuthorityBridgeError::ReplayedAuthorityReceipt)
    );

    // Execution at exact valid_until is rejected with ExpiredAtOrAfterUntil.
    cmd_env.execution_instant = valid_until;
    assert_eq!(
        admit_qac_simulator_command(
            &cmd_env,
            &grant_u,
            &policy_u_ok,
            &mut policy_u_ok.clock(),
            &mut qac_ledger
        ),
        Err(QacAuthorityBridgeError::CalibrationClock(
            CalibrationClockBridgeError::ExpiredAtOrAfterUntil
        ))
    );
}
