//! QAC authority-gated observation bridges and simulator actuation gate for Mode `0x0106`
//! (`haptic_mechanical_sample`, Schema 207) and Mode `0x0107` (`ultrasound_beam_summary`, Schema 208).
//!
//! Enforces strict separation between empirical observations (`Measurement`, `RawRf`,
//! `BasebandIq`, `BeamSummary`), biomechanical/receptor models (`ReceptorModel`), and hardware
//! actuation commands (`ActuatorCommand`, `TransmitCommand`) per C02-C07 and C02-C08.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use etalon_sense_core::observation::ObservationEnvelope;
use qualia_core_db::{
    governance::consent_contract::ConsentGrant,
    sensing::{
        acquisition::Rate,
        authorization::{
            IdentityAuthority, RevocationAuthority, SensingAction, SensingModality, SensingPermit,
            SensingScopeAuthority, TrustedUnixClock,
        },
        record::ResolvedFieldPayloadBinding,
        SemanticId,
    },
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
        emf_iq_dose_bio::{ComponentTargetedUncertainty, EmfAuthorityBridgeError},
    },
    BridgeError, BridgedObservation, ConsentBridgeAuthority,
};

pub const QAC_MODE_0106_HAPTIC_MECHANICAL: u16 = 0x0106;
pub const QAC_MODE_0107_ULTRASOUND_BEAM: u16 = 0x0107;

const QAC_OBS_RECEIPT_DOMAIN: &[u8] = b"qualia.sensing.bridge.qac.obs.receipt.v1";
const QAC_CMD_RECEIPT_DOMAIN: &[u8] = b"qualia.sensing.bridge.qac.cmd.receipt.v1";

/// Shared Schema 4 key 1 (`Provenance.role`) enumeration for bridge role separation (C02-C07).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeProvenanceRole {
    Measured,
    Model,
    Derived,
    Command,
    OpaquePreserved,
}

/// Fixed-capacity zero-heap replay ledger for QAC observation and command receipts.
#[derive(Clone, Debug)]
pub struct QacReceiptReplayLedger<const CAP: usize> {
    seen_digests: [[u8; 32]; CAP],
    len: usize,
}

impl<const CAP: usize> Default for QacReceiptReplayLedger<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> QacReceiptReplayLedger<CAP> {
    pub const fn new() -> Self {
        Self {
            seen_digests: [[0u8; 32]; CAP],
            len: 0,
        }
    }

    pub fn admit_once(&mut self, receipt_digest: [u8; 32]) -> Result<(), QacAuthorityBridgeError> {
        if receipt_digest == [0u8; 32] {
            return Err(QacAuthorityBridgeError::InvalidReceiptDigest);
        }
        let mut idx = 0usize;
        while idx < self.len {
            if self.seen_digests[idx] == receipt_digest {
                return Err(QacAuthorityBridgeError::ReplayedAuthorityReceipt);
            }
            idx += 1;
        }
        if self.len >= CAP {
            return Err(QacAuthorityBridgeError::ReplayWindowExhausted);
        }
        self.seen_digests[self.len] = receipt_digest;
        self.len += 1;
        Ok(())
    }
}

/// Ed25519-signed authority receipt for QAC observation bridges (Modes `0x0106` and `0x0107`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignedQacObservationReceipt {
    pub receipt_id: [u8; 32],
    pub mode_code: u16,
    pub modality: SensingModality,
    pub source_id: u64,
    pub actor_id: [u8; 32],
    pub purpose_hash: u64,
    pub action: SensingAction,
    pub payload_digest: [u8; 32],
    pub calibration_artifact_id: [u8; 32],
    pub valid_from_unix_secs: u64,
    pub valid_until_unix_secs: u64,
    pub issuer_public_key: [u8; 32],
    pub signature: [u8; 64],
}

impl SignedQacObservationReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn sign(
        signing_key: &SigningKey,
        receipt_id: [u8; 32],
        mode_code: u16,
        modality: SensingModality,
        source_id: u64,
        actor_id: [u8; 32],
        purpose_hash: u64,
        action: SensingAction,
        payload_digest: [u8; 32],
        calibration_artifact_id: [u8; 32],
        valid_from_unix_secs: u64,
        valid_until_unix_secs: u64,
    ) -> Result<Self, QacAuthorityBridgeError> {
        if receipt_id == [0u8; 32]
            || source_id == 0
            || actor_id == [0u8; 32]
            || purpose_hash == 0
            || payload_digest == [0u8; 32]
            || calibration_artifact_id == [0u8; 32]
            || valid_from_unix_secs >= valid_until_unix_secs
        {
            return Err(QacAuthorityBridgeError::InvalidReceiptFields);
        }
        let issuer_public_key = signing_key.verifying_key().to_bytes();
        let mut draft = Self {
            receipt_id,
            mode_code,
            modality,
            source_id,
            actor_id,
            purpose_hash,
            action,
            payload_digest,
            calibration_artifact_id,
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
        let mut buf = [0u8; 204];
        buf[0..32].copy_from_slice(&self.receipt_id);
        buf[32..34].copy_from_slice(&self.mode_code.to_le_bytes());
        buf[34] = modality_tag(self.modality);
        buf[35..43].copy_from_slice(&self.source_id.to_le_bytes());
        buf[43..75].copy_from_slice(&self.actor_id);
        buf[75..83].copy_from_slice(&self.purpose_hash.to_le_bytes());
        buf[83] = action_tag(self.action);
        buf[84..116].copy_from_slice(&self.payload_digest);
        buf[116..148].copy_from_slice(&self.calibration_artifact_id);
        buf[148..156].copy_from_slice(&self.valid_from_unix_secs.to_le_bytes());
        buf[156..164].copy_from_slice(&self.valid_until_unix_secs.to_le_bytes());
        buf[164..196].copy_from_slice(&self.issuer_public_key);
        buf[196..204].copy_from_slice(b"QAC_OBSV");
        blake3_domain_digest(QAC_OBS_RECEIPT_DOMAIN, &buf)
    }

    pub fn receipt_digest(&self) -> [u8; 32] {
        let mut buf = [0u8; 96];
        buf[0..32].copy_from_slice(&self.signing_digest());
        buf[32..96].copy_from_slice(&self.signature);
        blake3_domain_digest(b"qualia.sensing.bridge.qac.obs.receipt.id.v1", &buf)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn verify_against(
        &self,
        expected_mode: u16,
        expected_receipt_digest: [u8; 32],
        permit: &SensingPermit,
        expected_payload_digest: [u8; 32],
        expected_calibration_id: [u8; 32],
        now_unix_secs: u64,
    ) -> Result<(), QacAuthorityBridgeError> {
        if self.receipt_id == [0u8; 32]
            || self.valid_from_unix_secs >= self.valid_until_unix_secs
            || self.issuer_public_key == [0u8; 32]
        {
            return Err(QacAuthorityBridgeError::InvalidReceiptFields);
        }
        if self.mode_code != expected_mode {
            return Err(QacAuthorityBridgeError::ReceiptModeMismatch);
        }
        let req = permit.request();
        if self.modality != req.modality()
            || !matches!(
                req.modality(),
                SensingModality::Microphone | SensingModality::FieldInstrument
            )
        {
            return Err(QacAuthorityBridgeError::ModalityMismatch);
        }
        if self.action != SensingAction::Process || req.action() != SensingAction::Process {
            return Err(QacAuthorityBridgeError::ActionScopeMismatch);
        }
        if self.source_id != req.source_id()
            || self.actor_id != req.actor_id()
            || self.purpose_hash != req.purpose_hash()
        {
            return Err(QacAuthorityBridgeError::ReceiptPrincipalOrSourceMismatch);
        }
        if self.payload_digest != expected_payload_digest {
            return Err(QacAuthorityBridgeError::ReceiptPayloadDigestMismatch);
        }
        if self.calibration_artifact_id != expected_calibration_id {
            return Err(QacAuthorityBridgeError::ReceiptCalibrationDigestMismatch);
        }
        if now_unix_secs < self.valid_from_unix_secs || now_unix_secs >= self.valid_until_unix_secs
        {
            return Err(QacAuthorityBridgeError::ReceiptExpiredOrNotYetValid);
        }
        let vk = VerifyingKey::from_bytes(&self.issuer_public_key)
            .map_err(|_| QacAuthorityBridgeError::InvalidReceiptSignature)?;
        let sig = Signature::from_bytes(&self.signature);
        let msg = self.signing_digest();
        vk.verify_strict(&msg, &sig)
            .map_err(|_| QacAuthorityBridgeError::InvalidReceiptSignature)?;
        if self.receipt_digest() != expected_receipt_digest {
            return Err(QacAuthorityBridgeError::ReceiptDigestMismatch);
        }
        Ok(())
    }
}

/// QAC Mode `0x0106` haptic sample role (`haptic_mechanical_sample`, Schema 207 key 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QacHapticRole {
    Measurement,
    ReceptorModel,
    ActuatorCommand,
}

/// Sign convention for haptic contact force (Schema 207 key 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HapticForceConvention {
    ForceOnBody,
    ForceOnDevice,
}

/// Input contract for QAC Mode `0x0106` (`haptic_mechanical_sample`, Schema 207) observation bridge.
#[derive(Clone, Copy, Debug)]
pub struct QacMode0106HapticInput {
    pub record_id: u64,
    pub qualia_source_id: u64,
    pub observation: ObservationEnvelope,
    pub binding: ResolvedFieldPayloadBinding,
    pub payload_digest: [u8; 32],
    pub calibration_artifact_id: [u8; 32],
    pub calibration_window: BridgeCalibrationWindow,
    pub role: QacHapticRole,
    pub contact_frame_id: u32,
    pub force_convention: HapticForceConvention,
    pub force_vector_n: [f64; 3],
    pub displacement_vector_m: [f64; 3],
    pub contact_area_m2: f64,
    pub force_uncertainty: ComponentTargetedUncertainty,
    pub displacement_uncertainty: ComponentTargetedUncertainty,
    pub receptor_model_id: Option<SemanticId>,
    pub receipt: SignedQacObservationReceipt,
    pub expected_receipt_digest: [u8; 32],
}

/// Verified and bridged QAC Mode `0x0106` haptic observation or receptor-model record.
pub struct BridgedQacMode0106Haptic {
    pub bridged: BridgedObservation,
    pub observation_instant: BridgeRationalInstant,
    pub role: QacHapticRole,
    pub contact_frame_id: u32,
    pub force_convention: HapticForceConvention,
    pub force_vector_n: [f64; 3],
    pub displacement_vector_m: [f64; 3],
    pub contact_area_m2: f64,
    pub authority_receipt_digest: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
pub fn bridge_qac_mode0106_haptic_with_consent<A: ConsentBridgeAuthority, const CAP: usize>(
    authority: &A,
    permit: SensingPermit,
    grant: &ConsentGrant,
    identity: &impl IdentityAuthority,
    sensor_scope: &impl SensingScopeAuthority,
    revocations: &impl RevocationAuthority,
    clock: &mut impl TrustedUnixClock,
    replay_ledger: &mut QacReceiptReplayLedger<CAP>,
    input: QacMode0106HapticInput,
) -> Result<BridgedQacMode0106Haptic, QacAuthorityBridgeError> {
    match input.role {
        QacHapticRole::ActuatorCommand => {
            return Err(QacAuthorityBridgeError::CommandRoleRejectedInObservationBridge);
        }
        QacHapticRole::ReceptorModel => {
            let model_id = input
                .receptor_model_id
                .ok_or(QacAuthorityBridgeError::MissingReceptorOrThermalModel)?;
            if model_id.value().is_none() {
                return Err(QacAuthorityBridgeError::MissingReceptorOrThermalModel);
            }
        }
        QacHapticRole::Measurement => {}
    }
    if input.contact_frame_id == 0
        || !all_finite_3(input.force_vector_n)
        || !all_finite_3(input.displacement_vector_m)
        || !input.contact_area_m2.is_finite()
        || input.contact_area_m2 <= 0.0
    {
        return Err(QacAuthorityBridgeError::InvalidHapticGeometryOrForce);
    }
    input
        .force_uncertainty
        .validate()
        .map_err(QacAuthorityBridgeError::Uncertainty)?;
    input
        .displacement_uncertainty
        .validate()
        .map_err(QacAuthorityBridgeError::Uncertainty)?;

    let now_unix = clock
        .now_unix()
        .map_err(|_| QacAuthorityBridgeError::ClockUnavailable)?;
    input.receipt.verify_against(
        QAC_MODE_0106_HAPTIC_MECHANICAL,
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

    Ok(BridgedQacMode0106Haptic {
        bridged,
        observation_instant,
        role: input.role,
        contact_frame_id: input.contact_frame_id,
        force_convention: input.force_convention,
        force_vector_n: input.force_vector_n,
        displacement_vector_m: input.displacement_vector_m,
        contact_area_m2: input.contact_area_m2,
        authority_receipt_digest: input.expected_receipt_digest,
    })
}

#[path = "qac_ultrasound_actuation.rs"]
mod qac_ultrasound_actuation;
pub use qac_ultrasound_actuation::*;


const fn action_tag(action: SensingAction) -> u8 {
    match action {
        SensingAction::Acquire => 1,
        SensingAction::Process => 2,
        SensingAction::Store => 3,
        SensingAction::Export => 4,
        SensingAction::Share => 5,
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

fn all_finite_3(v: [f64; 3]) -> bool {
    v[0].is_finite() && v[1].is_finite() && v[2].is_finite()
}
