use super::actuation_gate::{
    admit_simulator_command, reject_passive_artifact_for_actuation,
    verify_empirical_join_admission, ActuationGateError, CommandReplayLedger, EpistemicClass,
    HapticSchema207Role, PassiveArtifactKind, ProvenanceRole, ProvenanceRoleRecord,
    SignedSimulatorCommandReceipt, SimulatorCommandClaims, SimulatorCommandKind,
    SimulatorCommandRequest, UltrasoundSchema208Role,
};
use super::certificates::{CalibrationError, CalibrationValidityWindow, RationalInstant};
use super::custody_bridge::{
    verify_acquisition_derivation_custody, verify_acquisition_original_custody,
    verify_and_consume_stream_receipt, AuthorityReceiptTarget, ConsentBoundSidecarAuthority,
    CustodyBridgeError, ExpectedStreamBinding, ReceiptReplayWindow, SignedAuthorityReceipt,
};
use super::tests::{sample_clock, FixedClock, FlagRevocation, TestIdentity};

use crate::governance::consent_contract::{ConsentGrant, ConsentScope};
use crate::sensing::acquisition::{ArtifactLink, Derivation, Loss, Original};
use crate::sensing::authorization::{
    authorize_sensing, AuthorizationError, SensingAction, SensingModality, SensingRequest,
};
use crate::sensing::sidecar_attestation::{SidecarReceipt, SidecarReceiptAuthority, SIDECAR_BYTES};
use crate::sensing::signed_scope::SignedSensingScope;
use crate::sensing::SemanticId;
use ed25519_dalek::SigningKey;

#[test]
fn custody_bridge_prevents_replay_substitution_revocation_and_action_escalation() {
    let principal_key = SigningKey::from_bytes(&[10u8; 32]);
    let actor_key = SigningKey::from_bytes(&[20u8; 32]);
    let receipt_signer_key = SigningKey::from_bytes(&[30u8; 32]);

    let principal_did = principal_key.verifying_key().to_bytes();
    let actor_did = actor_key.verifying_key().to_bytes();
    let now = 1_700_000_100;
    let grant = ConsentGrant::new_signed(
        [0xAA; 32],
        &principal_key,
        actor_did,
        ConsentScope(ConsentScope::VITALS),
        9001,
        now - 100,
        now + 3600,
        42,
    );

    let req_acquire = SensingRequest::new(
        actor_did,
        7001,
        SensingModality::Sdr,
        SensingAction::Acquire,
        9001,
    )
    .unwrap();
    let signed_scope = SignedSensingScope::sign(&grant, &principal_key, req_acquire).unwrap();
    let identity = TestIdentity {
        expected_principal: principal_did,
        expected_actor: actor_did,
    };
    let mut clock = FixedClock { now };
    let rev_ok = FlagRevocation { revoked: false };

    let permit = authorize_sensing(
        &grant,
        req_acquire,
        &identity,
        &signed_scope,
        &rev_ok,
        &mut clock,
    )
    .unwrap();
    let revalidated = permit
        .revalidate(&grant, &identity, &signed_scope, &rev_ok, &mut clock)
        .unwrap();

    let payload_bytes = b"exact-preserved-mode-0x0104-bytes";
    let original = Original::admit(payload_bytes, 1024).unwrap();
    let index_commitment = [0xCC; 32];

    let receipt = SignedAuthorityReceipt::issue(
        &revalidated,
        grant.expires_at,
        AuthorityReceiptTarget::ProvenanceSchema4,
        original.digest(),
        index_commitment,
        1,
        &receipt_signer_key,
    )
    .unwrap();
    let receipt_digest = receipt.receipt_digest();

    let mut replay_window = ReceiptReplayWindow::new();
    verify_acquisition_original_custody(
        original,
        index_commitment,
        receipt_digest,
        AuthorityReceiptTarget::ProvenanceSchema4,
        &receipt,
        req_acquire,
        &grant,
        &identity,
        &signed_scope,
        &rev_ok,
        &mut clock,
        &mut replay_window,
    )
    .unwrap();

    // 1. Replay of the same receipt fails closed
    assert_eq!(
        verify_acquisition_original_custody(
            original,
            index_commitment,
            receipt_digest,
            AuthorityReceiptTarget::ProvenanceSchema4,
            &receipt,
            req_acquire,
            &grant,
            &identity,
            &signed_scope,
            &rev_ok,
            &mut clock,
            &mut replay_window,
        ),
        Err(CustodyBridgeError::ReplayDetected)
    );

    // 2. Action escalation (Acquire receipt used for Export) fails closed
    let mut fresh_window = ReceiptReplayWindow::new();
    let req_export = SensingRequest::new(
        actor_did,
        7001,
        SensingModality::Sdr,
        SensingAction::Export,
        9001,
    )
    .unwrap();
    assert_eq!(
        verify_and_consume_stream_receipt(
            &receipt,
            &ExpectedStreamBinding {
                expected_target: AuthorityReceiptTarget::ProvenanceSchema4,
                stored_receipt_digest: receipt_digest,
                original_blake3: original.digest(),
                index_commitment,
                requested_operation: req_export,
            },
            &grant,
            &identity,
            &signed_scope,
            &rev_ok,
            &mut clock,
            &mut fresh_window,
        ),
        Err(CustodyBridgeError::ActionEscalation {
            granted: SensingAction::Acquire,
            requested: SensingAction::Export,
        })
    );

    // 3. Cross-stream substitution (wrong index_commitment) fails closed
    assert_eq!(
        verify_and_consume_stream_receipt(
            &receipt,
            &ExpectedStreamBinding {
                expected_target: AuthorityReceiptTarget::ProvenanceSchema4,
                stored_receipt_digest: receipt_digest,
                original_blake3: original.digest(),
                index_commitment: [0xDD; 32],
                requested_operation: req_acquire,
            },
            &grant,
            &identity,
            &signed_scope,
            &rev_ok,
            &mut clock,
            &mut fresh_window,
        ),
        Err(CustodyBridgeError::IndexCommitmentMismatch)
    );

    // 4. Wrong source_id fails closed
    let req_wrong_source = SensingRequest::new(
        actor_did,
        9999,
        SensingModality::Sdr,
        SensingAction::Acquire,
        9001,
    )
    .unwrap();
    assert_eq!(
        verify_and_consume_stream_receipt(
            &receipt,
            &ExpectedStreamBinding {
                expected_target: AuthorityReceiptTarget::ProvenanceSchema4,
                stored_receipt_digest: receipt_digest,
                original_blake3: original.digest(),
                index_commitment,
                requested_operation: req_wrong_source,
            },
            &grant,
            &identity,
            &signed_scope,
            &rev_ok,
            &mut clock,
            &mut fresh_window,
        ),
        Err(CustodyBridgeError::SourceIdMismatch)
    );

    // 5. Revoked grant fails closed
    let rev_true = FlagRevocation { revoked: true };
    assert_eq!(
        verify_and_consume_stream_receipt(
            &receipt,
            &ExpectedStreamBinding {
                expected_target: AuthorityReceiptTarget::ProvenanceSchema4,
                stored_receipt_digest: receipt_digest,
                original_blake3: original.digest(),
                index_commitment,
                requested_operation: req_acquire,
            },
            &grant,
            &identity,
            &signed_scope,
            &rev_true,
            &mut clock,
            &mut fresh_window,
        ),
        Err(CustodyBridgeError::Authorization(
            AuthorizationError::Revoked
        ))
    );

    // 6. Schema 7 Derivation custody verification + Sidecar attestation binding
    let req_process = SensingRequest::new(
        actor_did,
        7001,
        SensingModality::Sdr,
        SensingAction::Process,
        9001,
    )
    .unwrap();
    let scope_process = SignedSensingScope::sign(&grant, &principal_key, req_process).unwrap();
    let permit_proc = authorize_sensing(
        &grant,
        req_process,
        &identity,
        &scope_process,
        &rev_ok,
        &mut clock,
    )
    .unwrap();
    let reval_proc = permit_proc
        .revalidate(&grant, &identity, &scope_process, &rev_ok, &mut clock)
        .unwrap();
    let derived_output = Original::admit(b"derived-output-bytes", 1024).unwrap();
    let inputs = [original];
    let losses = [Loss::NoneDeclared];
    let derivation = Derivation::admit(
        derived_output,
        &inputs,
        ArtifactLink {
            digest: [0x11; 32],
            identifier: SemanticId::new(1).unwrap(),
        },
        ArtifactLink {
            digest: [0x22; 32],
            identifier: SemanticId::new(2).unwrap(),
        },
        &losses,
        8,
    )
    .unwrap();
    let deriv_receipt = SignedAuthorityReceipt::issue(
        &reval_proc,
        grant.expires_at,
        AuthorityReceiptTarget::AcquisitionDerivationSchema7,
        derived_output.digest(),
        index_commitment,
        2,
        &receipt_signer_key,
    )
    .unwrap();
    verify_acquisition_derivation_custody(
        derivation,
        index_commitment,
        deriv_receipt.receipt_digest(),
        &deriv_receipt,
        req_process,
        &grant,
        &identity,
        &scope_process,
        &rev_ok,
        &mut clock,
        &mut fresh_window,
    )
    .unwrap();

    let chain_id = [0x77; 32];
    let sidecar_auth = ConsentBoundSidecarAuthority::new(&deriv_receipt, chain_id).unwrap();
    let sidecar_bytes = [0u8; SIDECAR_BYTES];
    let sidecar_receipt = SidecarReceipt::sign(
        &sidecar_bytes,
        payload_bytes,
        1024,
        deriv_receipt.receipt_digest(),
        chain_id,
        0,
        None,
        &receipt_signer_key,
    )
    .unwrap();
    assert!(sidecar_auth.permits(&sidecar_receipt, &sidecar_bytes, payload_bytes));
}

#[test]
fn epistemic_class_separation_and_simulator_actuation_gate_enforce_c02_c07() {
    assert_eq!(
        verify_empirical_join_admission(
            ProvenanceRole::Measured,
            EpistemicClass::Empirical,
            ProvenanceRole::Derived,
            EpistemicClass::Empirical
        ),
        Ok(())
    );
    for forbidden_class in [
        EpistemicClass::Hypothesis,
        EpistemicClass::Counterfactual,
        EpistemicClass::Fictional,
        EpistemicClass::ModelEstimate,
    ] {
        assert_eq!(
            verify_empirical_join_admission(
                ProvenanceRole::Measured,
                EpistemicClass::Empirical,
                ProvenanceRole::Measured,
                forbidden_class
            ),
            Err(ActuationGateError::NonEmpiricalClassRejected(
                forbidden_class
            ))
        );
    }
    assert_eq!(
        verify_empirical_join_admission(
            ProvenanceRole::Command,
            EpistemicClass::Empirical,
            ProvenanceRole::Measured,
            EpistemicClass::Empirical
        ),
        Err(ActuationGateError::RoleForbiddenInEmpiricalJoin(
            ProvenanceRole::Command
        ))
    );

    for passive in [
        PassiveArtifactKind::IonizingDoseGyOrSv,
        PassiveArtifactKind::UltrasoundMechanicalOrThermalIndex,
        PassiveArtifactKind::MeasuredHapticForceOrIndentation,
        PassiveArtifactKind::PerceptualPitchOrVoiceFeature,
        PassiveArtifactKind::IntegrityCrcOrBlake3Digest,
        PassiveArtifactKind::PassiveConsentGrantId,
        PassiveArtifactKind::CalibrationCertificateDigest,
    ] {
        assert_eq!(
            reject_passive_artifact_for_actuation(passive),
            Err(ActuationGateError::PassiveObservationCannotAuthorizeActuation(passive))
        );
    }

    assert_eq!(
        SimulatorCommandKind::from_haptic_role(HapticSchema207Role::Measurement),
        Err(ActuationGateError::NonCommandDomainRoleRejected)
    );
    assert_eq!(
        SimulatorCommandKind::from_ultrasound_role(UltrasoundSchema208Role::BeamSummary),
        Err(ActuationGateError::NonCommandDomainRoleRejected)
    );
    assert_eq!(
        SimulatorCommandKind::from_haptic_role(HapticSchema207Role::ActuatorCommand),
        Ok(SimulatorCommandKind::HapticActuatorCommandSchema207)
    );
    assert_eq!(
        SimulatorCommandKind::from_ultrasound_role(UltrasoundSchema208Role::TransmitCommand),
        Ok(SimulatorCommandKind::UltrasoundTransmitCommandSchema208)
    );

    let measured_ok = ProvenanceRoleRecord {
        role: ProvenanceRole::Measured,
        source_digest: None,
        algorithm_id: SemanticId::Unknown,
        algorithm_version: SemanticId::Unknown,
        instrument_uri: SemanticId::new(10).unwrap(),
        authority_receipt_digest: None,
    };
    assert_eq!(measured_ok.validate(), Ok(()));

    let principal_key = SigningKey::from_bytes(&[41u8; 32]);
    let actor_key = SigningKey::from_bytes(&[42u8; 32]);
    let cmd_authority_key = SigningKey::from_bytes(&[43u8; 32]);
    let actor_did = actor_key.verifying_key().to_bytes();
    let now_unix = 1_700_000_500;
    let grant = ConsentGrant::new_signed(
        [0xBB; 32],
        &principal_key,
        actor_did,
        ConsentScope(ConsentScope::VITALS),
        777,
        now_unix - 60,
        now_unix + 600,
        99,
    );

    let clk = sample_clock(55, 1, 1_000);
    let t_0 = RationalInstant::from_clock_tick_subtick(clk, 100, 0, 1).unwrap();
    let t_exp = RationalInstant::from_clock_tick_subtick(clk, 200, 0, 1).unwrap();
    let cmd_window = CalibrationValidityWindow::new(t_0, t_exp).unwrap();
    let device_digest = [0xE1; 32];
    let frame_id = 7u32;

    let cmd_claims = SimulatorCommandClaims {
        command_id: [0xC1; 32],
        command_nonce: 1,
        command_kind: SimulatorCommandKind::HapticActuatorCommandSchema207,
        provenance_role: ProvenanceRole::Command,
        grant_id: grant.grant_id,
        grant_digest: grant.compute_digest(),
        actor_id: actor_did,
        source_id: 5001,
        device_profile_digest: device_digest,
        contact_or_array_frame_id: frame_id,
        validity_window: cmd_window,
    };
    let cmd_receipt = SignedSimulatorCommandReceipt::sign(cmd_claims, &cmd_authority_key).unwrap();
    let cmd_auth_pk = cmd_authority_key.verifying_key().to_bytes();
    let t_exec_valid = RationalInstant::from_clock_tick_subtick(clk, 199, 999, 1000).unwrap();

    let base_req = SimulatorCommandRequest {
        requested_kind: SimulatorCommandKind::HapticActuatorCommandSchema207,
        descriptor_provenance_role: ProvenanceRole::Command,
        stored_authority_receipt_digest: cmd_receipt.receipt_digest(),
        device_profile_digest: device_digest,
        contact_or_array_frame_id: frame_id,
        actor_id: actor_did,
        source_id: 5001,
        execution_instant: t_exec_valid,
    };

    let mut unix_clock = FixedClock { now: now_unix };
    let rev_ok = FlagRevocation { revoked: false };
    let mut cmd_ledger = CommandReplayLedger::new();

    // Non-command role (Measured) never executes even with valid receipt
    let mut measured_req = base_req;
    measured_req.descriptor_provenance_role = ProvenanceRole::Measured;
    assert_eq!(
        admit_simulator_command(
            &measured_req,
            &cmd_receipt,
            &cmd_auth_pk,
            &grant,
            &rev_ok,
            &mut unix_clock,
            &mut cmd_ledger
        ),
        Err(ActuationGateError::NonCommandProvenanceRoleRejected(
            ProvenanceRole::Measured
        ))
    );

    // Exact expiry boundary t == t_exp fails closed (zero tolerance)
    let mut expired_req = base_req;
    expired_req.execution_instant = t_exp;
    assert_eq!(
        admit_simulator_command(
            &expired_req,
            &cmd_receipt,
            &cmd_auth_pk,
            &grant,
            &rev_ok,
            &mut unix_clock,
            &mut cmd_ledger
        ),
        Err(ActuationGateError::CommandWindow(
            CalibrationError::CalibrationExpiredAtOrAfterUntil
        ))
    );

    // Wrong device profile digest fails closed
    let mut wrong_dev_req = base_req;
    wrong_dev_req.device_profile_digest = [0xFF; 32];
    assert_eq!(
        admit_simulator_command(
            &wrong_dev_req,
            &cmd_receipt,
            &cmd_auth_pk,
            &grant,
            &rev_ok,
            &mut unix_clock,
            &mut cmd_ledger
        ),
        Err(ActuationGateError::DeviceProfileDigestMismatch)
    );

    // Valid command at t_exec_valid succeeds once
    let admitted = admit_simulator_command(
        &base_req,
        &cmd_receipt,
        &cmd_auth_pk,
        &grant,
        &rev_ok,
        &mut unix_clock,
        &mut cmd_ledger,
    )
    .unwrap();
    assert_eq!(admitted.receipt_digest(), &cmd_receipt.receipt_digest());
    assert_eq!(admitted.contact_or_array_frame_id(), frame_id);

    // Replay of the same command receipt fails closed
    assert_eq!(
        admit_simulator_command(
            &base_req,
            &cmd_receipt,
            &cmd_auth_pk,
            &grant,
            &rev_ok,
            &mut unix_clock,
            &mut cmd_ledger
        ),
        Err(ActuationGateError::CommandReplayDetected)
    );
}
