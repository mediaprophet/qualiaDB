//! QAC Mode `0x0107` ultrasound beam summary bridge, passive observation separation,
//! simulator actuation gate, and error definitions.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use etalon_sense_core::observation::ObservationEnvelope;
use qualia_core_db::{
    governance::consent_contract::ConsentGrant,
    sensing::{
        acquisition::Rate,
        authorization::{
            IdentityAuthority, RevocationAuthority, SensingPermit, SensingScopeAuthority,
            TrustedUnixClock,
        },
        record::ResolvedFieldPayloadBinding,
        SemanticId,
    },
};

use super::{
    BridgeProvenanceRole, QacReceiptReplayLedger, SignedQacObservationReceipt,
    QAC_CMD_RECEIPT_DOMAIN, QAC_MODE_0107_ULTRASOUND_BEAM,
};
use crate::{
    bridge_observation_with_consent,
    emf_qac_authority::{
        calibration_clock_bridge::{
            apply_single_calibration_pair, blake3_domain_digest,
            compute_envelope_phase_uncertainty_rad, reject_unconjugated_iq_memcpy,
            BridgeCalibrationStage, BridgeCalibrationWindow, BridgeIqConvention,
            BridgeRationalInstant, CalibrationClockBridgeError,
        },
        emf_iq_dose_bio::EmfAuthorityBridgeError,
    },
    BridgeError, BridgedObservation, ConsentBridgeAuthority,
};

/// QAC Mode `0x0107` ultrasound record role (`ultrasound_beam_summary`, Schema 208 key 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QacUltrasoundRole {
    RawRf,
    BasebandIq,
    BeamSummary,
    TransmitCommand,
}

/// Ultrasound Thermal Index variant (`TIS`, `TIB`, `TIC` per Schema 208 key 10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThermalIndexVariant {
    Tis,
    Tib,
    Tic,
}

/// Input contract for QAC Mode `0x0107` (`ultrasound_beam_summary`, Schema 208) observation bridge.
#[derive(Clone, Copy, Debug)]
pub struct QacMode0107UltrasoundInput {
    pub record_id: u64,
    pub qualia_source_id: u64,
    pub observation: ObservationEnvelope,
    pub binding: ResolvedFieldPayloadBinding,
    pub payload_digest: [u8; 32],
    pub calibration_artifact_id: [u8; 32],
    pub calibration_window: BridgeCalibrationWindow,
    pub role: QacUltrasoundRole,
    pub array_frame_id: u32,
    pub center_frequency_hz: f64,
    pub rf_sample_rate: Rate,
    pub prf_rate: Rate,
    pub iq_convention: BridgeIqConvention,
    pub calibration_stage: BridgeCalibrationStage,
    pub scale_factor: f64,
    pub offset: f64,
    pub raw_iq_or_rf_pair: (f64, f64),
    pub max_phase_uncertainty_rad: f64,
    pub derated_peak_rarefactional_pressure_mpa: f64,
    pub mechanical_index: f64,
    pub thermal_index: Option<f64>,
    pub thermal_index_variant: Option<ThermalIndexVariant>,
    pub thermal_model_id: Option<SemanticId>,
    pub receipt: SignedQacObservationReceipt,
    pub expected_receipt_digest: [u8; 32],
}

/// Verified and bridged QAC Mode `0x0107` ultrasound observation record.
pub struct BridgedQacMode0107Ultrasound {
    pub bridged: BridgedObservation,
    pub observation_instant: BridgeRationalInstant,
    pub role: QacUltrasoundRole,
    pub array_frame_id: u32,
    pub center_frequency_hz: f64,
    pub rf_sample_rate: Rate,
    pub prf_rate: Rate,
    pub iq_convention: BridgeIqConvention,
    pub calibrated_sample_pair: (f64, f64),
    pub resulting_stage: BridgeCalibrationStage,
    pub phase_uncertainty_rad: f64,
    pub derated_peak_rarefactional_pressure_mpa: f64,
    pub mechanical_index: f64,
    pub thermal_index: Option<f64>,
    pub thermal_index_variant: Option<ThermalIndexVariant>,
    pub authority_receipt_digest: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
pub fn bridge_qac_mode0107_ultrasound_with_consent<A: ConsentBridgeAuthority, const CAP: usize>(
    authority: &A,
    permit: SensingPermit,
    grant: &ConsentGrant,
    identity: &impl IdentityAuthority,
    sensor_scope: &impl SensingScopeAuthority,
    revocations: &impl RevocationAuthority,
    clock: &mut impl TrustedUnixClock,
    replay_ledger: &mut QacReceiptReplayLedger<CAP>,
    input: QacMode0107UltrasoundInput,
) -> Result<BridgedQacMode0107Ultrasound, QacAuthorityBridgeError> {
    if input.role == QacUltrasoundRole::TransmitCommand {
        return Err(QacAuthorityBridgeError::CommandRoleRejectedInObservationBridge);
    }
    if input.array_frame_id == 0
        || !input.center_frequency_hz.is_finite()
        || input.center_frequency_hz <= 0.0
        || !input.max_phase_uncertainty_rad.is_finite()
        || input.max_phase_uncertainty_rad <= 0.0
        || !input.derated_peak_rarefactional_pressure_mpa.is_finite()
        || input.derated_peak_rarefactional_pressure_mpa < 0.0
        || !input.mechanical_index.is_finite()
        || input.mechanical_index < 0.0
    {
        return Err(QacAuthorityBridgeError::InvalidUltrasoundParameters);
    }
    let rf_fs_hz = (input.rf_sample_rate.numerator as f64) / (input.rf_sample_rate.denominator as f64);
    let prf_hz = (input.prf_rate.numerator as f64) / (input.prf_rate.denominator as f64);
    if !rf_fs_hz.is_finite()
        || rf_fs_hz <= 0.0
        || !prf_hz.is_finite()
        || prf_hz <= 0.0
        || (prf_hz - input.center_frequency_hz).abs() < 1e-6
        || (prf_hz - rf_fs_hz).abs() < 1e-6
    {
        return Err(QacAuthorityBridgeError::PrfConflatedWithCarrierOrSampleRate);
    }
    reject_unconjugated_iq_memcpy(input.iq_convention, BridgeIqConvention::QacPositiveCarrier)
        .map_err(QacAuthorityBridgeError::CalibrationClock)?;

    if let Some(ti) = input.thermal_index {
        if !ti.is_finite() || ti < 0.0 {
            return Err(QacAuthorityBridgeError::InvalidUltrasoundParameters);
        }
        if input.thermal_index_variant.is_none() {
            return Err(QacAuthorityBridgeError::MissingReceptorOrThermalModel);
        }
        let model_id = input
            .thermal_model_id
            .ok_or(QacAuthorityBridgeError::MissingReceptorOrThermalModel)?;
        if model_id.value().is_none() {
            return Err(QacAuthorityBridgeError::MissingReceptorOrThermalModel);
        }
    }

    let now_unix = clock
        .now_unix()
        .map_err(|_| QacAuthorityBridgeError::ClockUnavailable)?;
    input.receipt.verify_against(
        QAC_MODE_0107_ULTRASOUND_BEAM,
        input.expected_receipt_digest,
        &permit,
        input.payload_digest,
        input.calibration_artifact_id,
        now_unix,
    )?;

    let observation_instant = input
        .calibration_window
        .verify_observation_calibration(&input.observation, &input.calibration_artifact_id)
        .map_err(QacAuthorityBridgeError::CalibrationClock)?;

    let phase_uncertainty_rad =
        compute_envelope_phase_uncertainty_rad(&input.observation, input.center_frequency_hz, 0.0)
            .map_err(QacAuthorityBridgeError::CalibrationClock)?;
    if phase_uncertainty_rad > input.max_phase_uncertainty_rad {
        return Err(QacAuthorityBridgeError::CalibrationClock(
            CalibrationClockBridgeError::PhaseUncertaintyExceedsBudget,
        ));
    }

    let is_complex = input.role == QacUltrasoundRole::BasebandIq;
    let (calibrated_sample_pair, resulting_stage, _, _) = apply_single_calibration_pair(
        input.calibration_stage,
        input.raw_iq_or_rf_pair,
        input.scale_factor,
        input.offset,
        is_complex,
    )
    .map_err(QacAuthorityBridgeError::CalibrationClock)?;

    let bridged = bridge_observation_with_consent(
        authority,
        permit,
        grant,
        identity,
        sensor_scope,
        revocations,
        clock,
        input.record_id,
        input.qualia_source_id,
        input.observation,
        input.binding,
    )
    .map_err(QacAuthorityBridgeError::Bridge)?;

    replay_ledger.admit_once(input.expected_receipt_digest)?;

    Ok(BridgedQacMode0107Ultrasound {
        bridged,
        observation_instant,
        role: input.role,
        array_frame_id: input.array_frame_id,
        center_frequency_hz: input.center_frequency_hz,
        rf_sample_rate: input.rf_sample_rate,
        prf_rate: input.prf_rate,
        iq_convention: input.iq_convention,
        calibrated_sample_pair,
        resulting_stage,
        phase_uncertainty_rad,
        derated_peak_rarefactional_pressure_mpa: input.derated_peak_rarefactional_pressure_mpa,
        mechanical_index: input.mechanical_index,
        thermal_index: input.thermal_index,
        thermal_index_variant: input.thermal_index_variant,
        authority_receipt_digest: input.expected_receipt_digest,
    })
}

/// Passive observation artifacts that must NEVER be accepted as simulator actuation authority (C02-C07).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassiveObservationArtifactKind {
    StoredIonizingDoseGyOrSv,
    StoredUltrasoundMechanicalIndex,
    StoredUltrasoundThermalIndex,
    StoredHapticContactForceN,
    StoredPitchOrVoiceFeature,
    StoredContainerCrcOrBlake3Digest,
    StoredCalibrationCertificateDigest,
    PassiveObservationConsentId,
}

/// Unconditionally reject any passive observation/safety/checksum artifact presented for actuation.
pub const fn reject_passive_artifact_for_actuation(
    _artifact: PassiveObservationArtifactKind,
) -> Result<(), QacAuthorityBridgeError> {
    Err(QacAuthorityBridgeError::PassiveObservationCannotAuthorizeActuation)
}

/// Explicit simulator command scopes (Schema 207 key 1 `actuator-command`, Schema 208 key 1 `transmit-command`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QacCommandScope {
    HapticActuatorCommandSchema207,
    UltrasoundTransmitCommandSchema208,
}

/// Ed25519-signed command receipt binding a simulator actuation command to device profile,
/// contact/array frame, command payload digest, and half-open validity window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignedQacCommandReceipt {
    pub receipt_id: [u8; 32],
    pub scope: QacCommandScope,
    pub source_id: u64,
    pub actor_id: [u8; 32],
    pub purpose_hash: u64,
    pub device_profile_digest: [u8; 32],
    pub contact_or_array_frame_id: u32,
    pub command_payload_digest: [u8; 32],
    pub valid_from_unix_secs: u64,
    pub valid_until_unix_secs: u64,
    pub issuer_public_key: [u8; 32],
    pub signature: [u8; 64],
}

impl SignedQacCommandReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn sign(
        signing_key: &SigningKey,
        receipt_id: [u8; 32],
        scope: QacCommandScope,
        source_id: u64,
        actor_id: [u8; 32],
        purpose_hash: u64,
        device_profile_digest: [u8; 32],
        contact_or_array_frame_id: u32,
        command_payload_digest: [u8; 32],
        valid_from_unix_secs: u64,
        valid_until_unix_secs: u64,
    ) -> Result<Self, QacAuthorityBridgeError> {
        if receipt_id == [0u8; 32]
            || source_id == 0
            || actor_id == [0u8; 32]
            || purpose_hash == 0
            || device_profile_digest == [0u8; 32]
            || contact_or_array_frame_id == 0
            || command_payload_digest == [0u8; 32]
            || valid_from_unix_secs >= valid_until_unix_secs
        {
            return Err(QacAuthorityBridgeError::InvalidReceiptFields);
        }
        let issuer_public_key = signing_key.verifying_key().to_bytes();
        let mut draft = Self {
            receipt_id,
            scope,
            source_id,
            actor_id,
            purpose_hash,
            device_profile_digest,
            contact_or_array_frame_id,
            command_payload_digest,
            valid_from_unix_secs,
            valid_until_unix_secs,
            issuer_public_key,
            signature: [0u8; 64],
        };
        let digest = draft.signing_digest();
        let sig: Signature = signing_key.sign(&digest);
        draft.signature = sig.to_bytes();
        Ok(draft)
    }

    pub fn signing_digest(&self) -> [u8; 32] {
        let mut buf = [0u8; 205];
        buf[0..32].copy_from_slice(&self.receipt_id);
        buf[32] = match self.scope {
            QacCommandScope::HapticActuatorCommandSchema207 => 1,
            QacCommandScope::UltrasoundTransmitCommandSchema208 => 2,
        };
        buf[33..41].copy_from_slice(&self.source_id.to_le_bytes());
        buf[41..73].copy_from_slice(&self.actor_id);
        buf[73..81].copy_from_slice(&self.purpose_hash.to_le_bytes());
        buf[81..113].copy_from_slice(&self.device_profile_digest);
        buf[113..117].copy_from_slice(&self.contact_or_array_frame_id.to_le_bytes());
        buf[117..149].copy_from_slice(&self.command_payload_digest);
        buf[149..157].copy_from_slice(&self.valid_from_unix_secs.to_le_bytes());
        buf[157..165].copy_from_slice(&self.valid_until_unix_secs.to_le_bytes());
        buf[165..197].copy_from_slice(&self.issuer_public_key);
        buf[197..205].copy_from_slice(b"QAC_CMD1");
        blake3_domain_digest(QAC_CMD_RECEIPT_DOMAIN, &buf)
    }

    pub fn receipt_digest(&self) -> [u8; 32] {
        let mut buf = [0u8; 96];
        buf[0..32].copy_from_slice(&self.signing_digest());
        buf[32..96].copy_from_slice(&self.signature);
        blake3_domain_digest(b"qualia.sensing.bridge.qac.cmd.receipt.id.v1", &buf)
    }
}

/// Simulator command envelope requiring explicit `BridgeProvenanceRole::Command`, live unrevoked consent,
/// device profile digest, contact/array frame ID, and half-open `[valid_from, valid_until)` expiry.
#[derive(Clone, Copy, Debug)]
pub struct QacSimulatorCommandEnvelope {
    pub scope: QacCommandScope,
    pub provenance_role: BridgeProvenanceRole,
    pub device_profile_digest: [u8; 32],
    pub contact_or_array_frame_id: u32,
    pub command_payload_digest: [u8; 32],
    pub stored_authority_receipt_digest: [u8; 32],
    pub execution_instant: BridgeRationalInstant,
    pub command_validity_window: BridgeCalibrationWindow,
    pub receipt: SignedQacCommandReceipt,
}

/// Verified simulator actuation command token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedQacSimulatorCommand {
    pub scope: QacCommandScope,
    pub authority_receipt_digest: [u8; 32],
    pub device_profile_digest: [u8; 32],
    pub contact_or_array_frame_id: u32,
    pub execution_instant: BridgeRationalInstant,
}

pub fn admit_qac_simulator_command<const CAP: usize>(
    envelope: &QacSimulatorCommandEnvelope,
    grant: &ConsentGrant,
    revocations: &impl RevocationAuthority,
    clock: &mut impl TrustedUnixClock,
    replay_ledger: &mut QacReceiptReplayLedger<CAP>,
) -> Result<VerifiedQacSimulatorCommand, QacAuthorityBridgeError> {
    if envelope.provenance_role != BridgeProvenanceRole::Command {
        return Err(QacAuthorityBridgeError::PassiveObservationCannotAuthorizeActuation);
    }
    if envelope.device_profile_digest == [0u8; 32] || envelope.contact_or_array_frame_id == 0 {
        return Err(QacAuthorityBridgeError::MissingDeviceProfileOrCoordinateFrame);
    }
    envelope
        .command_validity_window
        .verify_contains(envelope.execution_instant)
        .map_err(QacAuthorityBridgeError::CalibrationClock)?;

    let now_unix = clock
        .now_unix()
        .map_err(|_| QacAuthorityBridgeError::ClockUnavailable)?;
    if now_unix < grant.created_at || grant.verify(now_unix).is_err() {
        return Err(QacAuthorityBridgeError::ReceiptExpiredOrNotYetValid);
    }
    let grant_digest = grant.compute_digest();
    let revoked = revocations
        .is_revoked(&grant.grant_id, &grant_digest, now_unix)
        .map_err(|_| QacAuthorityBridgeError::RevocationCheckFailed)?;
    if revoked {
        return Err(QacAuthorityBridgeError::GrantRevoked);
    }

    let r = &envelope.receipt;
    if r.scope != envelope.scope
        || r.device_profile_digest != envelope.device_profile_digest
        || r.contact_or_array_frame_id != envelope.contact_or_array_frame_id
        || r.command_payload_digest != envelope.command_payload_digest
        || r.actor_id != grant.recipient_did
    {
        return Err(QacAuthorityBridgeError::CommandReceiptBindingMismatch);
    }
    if now_unix < r.valid_from_unix_secs || now_unix >= r.valid_until_unix_secs {
        return Err(QacAuthorityBridgeError::ReceiptExpiredOrNotYetValid);
    }
    let vk = VerifyingKey::from_bytes(&r.issuer_public_key)
        .map_err(|_| QacAuthorityBridgeError::InvalidReceiptSignature)?;
    let sig = Signature::from_bytes(&r.signature);
    let msg = r.signing_digest();
    vk.verify_strict(&msg, &sig)
        .map_err(|_| QacAuthorityBridgeError::InvalidReceiptSignature)?;
    let computed_digest = r.receipt_digest();
    if computed_digest != envelope.stored_authority_receipt_digest {
        return Err(QacAuthorityBridgeError::ReceiptDigestMismatch);
    }

    replay_ledger.admit_once(computed_digest)?;

    Ok(VerifiedQacSimulatorCommand {
        scope: envelope.scope,
        authority_receipt_digest: computed_digest,
        device_profile_digest: envelope.device_profile_digest,
        contact_or_array_frame_id: envelope.contact_or_array_frame_id,
        execution_instant: envelope.execution_instant,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QacAuthorityBridgeError {
    InvalidReceiptDigest,
    InvalidReceiptFields,
    InvalidReceiptSignature,
    ReceiptModeMismatch,
    ModalityMismatch,
    ActionScopeMismatch,
    ReceiptPrincipalOrSourceMismatch,
    ReceiptPayloadDigestMismatch,
    ReceiptCalibrationDigestMismatch,
    ReceiptExpiredOrNotYetValid,
    ReceiptDigestMismatch,
    ReplayedAuthorityReceipt,
    ReplayWindowExhausted,
    ClockUnavailable,
    CommandRoleRejectedInObservationBridge,
    MissingReceptorOrThermalModel,
    InvalidHapticGeometryOrForce,
    InvalidUltrasoundParameters,
    PrfConflatedWithCarrierOrSampleRate,
    PassiveObservationCannotAuthorizeActuation,
    MissingDeviceProfileOrCoordinateFrame,
    CommandReceiptBindingMismatch,
    RevocationCheckFailed,
    GrantRevoked,
    Uncertainty(EmfAuthorityBridgeError),
    CalibrationClock(CalibrationClockBridgeError),
    Bridge(BridgeError),
}
