//! EMF authority-gated observation bridges for Mode `0x0105` (`calibrated_iq_stft_v1`),
//! Mode `0x0106` (`ionizing_radiation_v1`), and Mode `0x0107` (`bioelectric_dipole_v1`).
//!
//! Enforces anti-substitution receipt binding, replay rejection, half-open rational
//! calibration validity `[valid_from, valid_until)` (C02-C03), single-application affine
//! calibration (C02-C05), EMF negative-carrier convention (`I_plus_iQ_exp_minus_iwt`, C02-C08),
//! component-targeted uncertainty on mixed-unit records, attenuation-vs-dose separation,
//! and bioelectric dipole consent-receipt & non-singular observation distance verification.

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
    emf_qac_authority::calibration_clock_bridge::{
        apply_single_calibration_pair, blake3_domain_digest, compute_envelope_phase_uncertainty_rad,
        reject_unconjugated_iq_memcpy, BridgeCalibrationStage, BridgeCalibrationWindow,
        BridgeIqConvention, BridgeRationalInstant, CalibrationClockBridgeError,
    },
    BridgeError, BridgedObservation, ConsentBridgeAuthority,
};

#[path = "emf_iq_dose_bio_modes.rs"]
mod emf_iq_dose_bio_modes;
pub use emf_iq_dose_bio_modes::*;

pub const EMF_MODE_0105_CALIBRATED_IQ_STFT: u16 = 0x0105;
pub const EMF_MODE_0106_IONIZING_RADIATION: u16 = 0x0106;
pub const EMF_MODE_0107_BIOELECTRIC_DIPOLE: u16 = 0x0107;

const EMF_RECEIPT_DOMAIN: &[u8] = b"qualia.sensing.bridge.emf.receipt.v1";

/// Fixed-capacity zero-heap replay window for EMF bridge receipts.
#[derive(Clone, Debug)]
pub struct EmfBridgeReceiptReplayWindow<const CAP: usize> {
    seen_digests: [[u8; 32]; CAP],
    len: usize,
}

impl<const CAP: usize> Default for EmfBridgeReceiptReplayWindow<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> EmfBridgeReceiptReplayWindow<CAP> {
    pub const fn new() -> Self {
        Self {
            seen_digests: [[0u8; 32]; CAP],
            len: 0,
        }
    }

    pub fn admit_once(&mut self, receipt_digest: [u8; 32]) -> Result<(), EmfAuthorityBridgeError> {
        if receipt_digest == [0u8; 32] {
            return Err(EmfAuthorityBridgeError::InvalidReceiptDigest);
        }
        let mut idx = 0usize;
        while idx < self.len {
            if self.seen_digests[idx] == receipt_digest {
                return Err(EmfAuthorityBridgeError::ReplayedAuthorityReceipt);
            }
            idx += 1;
        }
        if self.len >= CAP {
            return Err(EmfAuthorityBridgeError::ReplayWindowExhausted);
        }
        self.seen_digests[self.len] = receipt_digest;
        self.len += 1;
        Ok(())
    }
}

/// Ed25519-signed authority receipt binding an EMF mode (`0x0105`, `0x0106`, `0x0107`),
/// `source_id`, `actor_id` (`[u8; 32]`), `purpose_hash`, `SensingAction`, payload BLAKE3 digest,
/// calibration artifact digest, and half-open Unix validity `[valid_from_unix_secs, valid_until_unix_secs)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignedEmfBridgeReceipt {
    pub receipt_id: [u8; 32],
    pub mode_code: u16,
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

impl SignedEmfBridgeReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn sign(
        signing_key: &SigningKey,
        receipt_id: [u8; 32],
        mode_code: u16,
        source_id: u64,
        actor_id: [u8; 32],
        purpose_hash: u64,
        action: SensingAction,
        payload_digest: [u8; 32],
        calibration_artifact_id: [u8; 32],
        valid_from_unix_secs: u64,
        valid_until_unix_secs: u64,
    ) -> Result<Self, EmfAuthorityBridgeError> {
        if receipt_id == [0u8; 32]
            || source_id == 0
            || actor_id == [0u8; 32]
            || purpose_hash == 0
            || payload_digest == [0u8; 32]
            || calibration_artifact_id == [0u8; 32]
            || valid_from_unix_secs >= valid_until_unix_secs
        {
            return Err(EmfAuthorityBridgeError::InvalidReceiptFields);
        }
        let issuer_public_key = signing_key.verifying_key().to_bytes();
        let mut draft = Self {
            receipt_id,
            mode_code,
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
        let mut buf = [0u8; 203];
        buf[0..32].copy_from_slice(&self.receipt_id);
        buf[32..34].copy_from_slice(&self.mode_code.to_le_bytes());
        buf[34..42].copy_from_slice(&self.source_id.to_le_bytes());
        buf[42..74].copy_from_slice(&self.actor_id);
        buf[74..82].copy_from_slice(&self.purpose_hash.to_le_bytes());
        buf[82] = action_tag(self.action);
        buf[83..115].copy_from_slice(&self.payload_digest);
        buf[115..147].copy_from_slice(&self.calibration_artifact_id);
        buf[147..155].copy_from_slice(&self.valid_from_unix_secs.to_le_bytes());
        buf[155..163].copy_from_slice(&self.valid_until_unix_secs.to_le_bytes());
        buf[163..195].copy_from_slice(&self.issuer_public_key);
        buf[195..203].copy_from_slice(b"EMF_AUTH");
        blake3_domain_digest(EMF_RECEIPT_DOMAIN, &buf)
    }

    pub fn receipt_digest(&self) -> [u8; 32] {
        let mut buf = [0u8; 96];
        buf[0..32].copy_from_slice(&self.signing_digest());
        buf[32..96].copy_from_slice(&self.signature);
        blake3_domain_digest(b"qualia.sensing.bridge.emf.receipt.id.v1", &buf)
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
    ) -> Result<(), EmfAuthorityBridgeError> {
        if self.receipt_id == [0u8; 32]
            || self.valid_from_unix_secs >= self.valid_until_unix_secs
            || self.issuer_public_key == [0u8; 32]
        {
            return Err(EmfAuthorityBridgeError::InvalidReceiptFields);
        }
        if self.mode_code != expected_mode {
            return Err(EmfAuthorityBridgeError::ReceiptModeMismatch);
        }
        let req = permit.request();
        if !is_emf_sensing_modality(req.modality()) {
            return Err(EmfAuthorityBridgeError::ModalityMismatch);
        }
        if self.action != SensingAction::Process || req.action() != SensingAction::Process {
            return Err(EmfAuthorityBridgeError::ActionScopeMismatch);
        }
        if self.source_id != req.source_id()
            || self.actor_id != req.actor_id()
            || self.purpose_hash != req.purpose_hash()
        {
            return Err(EmfAuthorityBridgeError::ReceiptPrincipalOrSourceMismatch);
        }
        if self.payload_digest != expected_payload_digest {
            return Err(EmfAuthorityBridgeError::ReceiptPayloadDigestMismatch);
        }
        if self.calibration_artifact_id != expected_calibration_id {
            return Err(EmfAuthorityBridgeError::ReceiptCalibrationDigestMismatch);
        }
        if now_unix_secs < self.valid_from_unix_secs || now_unix_secs >= self.valid_until_unix_secs
        {
            return Err(EmfAuthorityBridgeError::ReceiptExpiredOrNotYetValid);
        }
        let vk = VerifyingKey::from_bytes(&self.issuer_public_key)
            .map_err(|_| EmfAuthorityBridgeError::InvalidReceiptSignature)?;
        let sig = Signature::from_bytes(&self.signature);
        let msg = self.signing_digest();
        vk.verify_strict(&msg, &sig)
            .map_err(|_| EmfAuthorityBridgeError::InvalidReceiptSignature)?;
        if self.receipt_digest() != expected_receipt_digest {
            return Err(EmfAuthorityBridgeError::ReceiptDigestMismatch);
        }
        Ok(())
    }
}

/// Input contract for EMF Mode `0x0105` (`calibrated_iq_stft_v1`, Schema 136).
#[derive(Clone, Copy, Debug)]
pub struct EmfMode0105IqInput {
    pub record_id: u64,
    pub qualia_source_id: u64,
    pub observation: ObservationEnvelope,
    pub binding: ResolvedFieldPayloadBinding,
    pub payload_digest: [u8; 32],
    pub calibration_artifact_id: [u8; 32],
    pub calibration_window: BridgeCalibrationWindow,
    pub calibration_stage: BridgeCalibrationStage,
    pub scale_factor: f64,
    pub offset: f64,
    pub raw_iq_sample: (f64, f64),
    pub carrier_hz: f64,
    pub baseband_sample_rate: Rate,
    pub lo_phase_rad: f64,
    pub iq_convention: BridgeIqConvention,
    pub max_phase_uncertainty_rad: f64,
    pub receipt: SignedEmfBridgeReceipt,
    pub expected_receipt_digest: [u8; 32],
}

/// Verified and bridged EMF Mode `0x0105` record preserving the original `ObservationEnvelope`,
/// calibrated I/Q sample in physical base units, carrier vs baseband rate, and phase uncertainty.
pub struct BridgedEmfMode0105Iq {
    pub bridged: BridgedObservation,
    pub observation_instant: BridgeRationalInstant,
    pub calibrated_iq_v_per_m: (f64, f64),
    pub resulting_stage: BridgeCalibrationStage,
    pub resulting_units_per_sample: f64,
    pub resulting_offset: f64,
    pub carrier_hz: f64,
    pub baseband_sample_rate: Rate,
    pub lo_phase_rad: f64,
    pub iq_convention: BridgeIqConvention,
    pub phase_uncertainty_rad: f64,
    pub authority_receipt_digest: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
pub fn bridge_emf_mode0105_iq_with_consent<A: ConsentBridgeAuthority, const CAP: usize>(
    authority: &A,
    permit: SensingPermit,
    grant: &ConsentGrant,
    identity: &impl IdentityAuthority,
    sensor_scope: &impl SensingScopeAuthority,
    revocations: &impl RevocationAuthority,
    clock: &mut impl TrustedUnixClock,
    replay_window: &mut EmfBridgeReceiptReplayWindow<CAP>,
    input: EmfMode0105IqInput,
) -> Result<BridgedEmfMode0105Iq, EmfAuthorityBridgeError> {
    if !input.carrier_hz.is_finite()
        || input.carrier_hz <= 0.0
        || !input.lo_phase_rad.is_finite()
        || !input.max_phase_uncertainty_rad.is_finite()
        || input.max_phase_uncertainty_rad <= 0.0
    {
        return Err(EmfAuthorityBridgeError::InvalidCarrierOrBasebandRate);
    }
    let fs_hz =
        (input.baseband_sample_rate.numerator as f64) / (input.baseband_sample_rate.denominator as f64);
    if !fs_hz.is_finite() || fs_hz <= 0.0 || (fs_hz - input.carrier_hz).abs() < 1e-6 {
        return Err(EmfAuthorityBridgeError::CarrierConflatedWithBasebandRate);
    }
    reject_unconjugated_iq_memcpy(input.iq_convention, BridgeIqConvention::EmfNegativeCarrier)
        .map_err(EmfAuthorityBridgeError::CalibrationClock)?;

    let now_unix = clock
        .now_unix()
        .map_err(|_| EmfAuthorityBridgeError::ClockUnavailable)?;
    input.receipt.verify_against(
        EMF_MODE_0105_CALIBRATED_IQ_STFT,
        input.expected_receipt_digest,
        &permit,
        input.payload_digest,
        input.calibration_artifact_id,
        now_unix,
    )?;

    let observation_instant = input
        .calibration_window
        .verify_observation_calibration(&input.observation, &input.calibration_artifact_id)
        .map_err(EmfAuthorityBridgeError::CalibrationClock)?;

    let phase_uncertainty_rad =
        compute_envelope_phase_uncertainty_rad(&input.observation, input.carrier_hz, 0.0)
            .map_err(EmfAuthorityBridgeError::CalibrationClock)?;
    if phase_uncertainty_rad > input.max_phase_uncertainty_rad {
        return Err(EmfAuthorityBridgeError::CalibrationClock(
            CalibrationClockBridgeError::PhaseUncertaintyExceedsBudget,
        ));
    }

    let (calibrated_iq_v_per_m, resulting_stage, resulting_units_per_sample, resulting_offset) =
        apply_single_calibration_pair(
            input.calibration_stage,
            input.raw_iq_sample,
            input.scale_factor,
            input.offset,
            true,
        )
        .map_err(EmfAuthorityBridgeError::CalibrationClock)?;

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
    .map_err(EmfAuthorityBridgeError::Bridge)?;

    replay_window.admit_once(input.expected_receipt_digest)?;

    Ok(BridgedEmfMode0105Iq {
        bridged,
        observation_instant,
        calibrated_iq_v_per_m,
        resulting_stage,
        resulting_units_per_sample,
        resulting_offset,
        carrier_hz: input.carrier_hz,
        baseband_sample_rate: input.baseband_sample_rate,
        lo_phase_rad: input.lo_phase_rad,
        iq_convention: input.iq_convention,
        phase_uncertainty_rad,
        authority_receipt_digest: input.expected_receipt_digest,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmfAuthorityBridgeError {
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
    MissingUncertaintyTargetQuantityOrUnit,
    InvalidUncertaintyValue,
    UnlabeledMixedUnitUncertaintyRejected,
    MixedUnitComponentsShareIdenticalUnit,
    NonEmpiricalEpistemicClassRejected,
    InvalidCarrierOrBasebandRate,
    CarrierConflatedWithBasebandRate,
    InvalidIonizingQuantity,
    DoseInferredFromAttenuationRejected,
    MissingDoseWeightingModel,
    MissingAnatomyOntologyBinding,
    InvalidBioDipoleGeometry,
    SingularDipoleSourceEvaluationRejected,
    MissingElectrodeObservationPosition,
    MissingReferencePotential,
    CalibrationClock(CalibrationClockBridgeError),
    Bridge(BridgeError),
}

const fn action_tag(action: SensingAction) -> u8 {
    match action {
        SensingAction::Acquire => 1,
        SensingAction::Process => 2,
        SensingAction::Store => 3,
        SensingAction::Export => 4,
        SensingAction::Share => 5,
    }
}

const fn is_emf_sensing_modality(modality: SensingModality) -> bool {
    matches!(
        modality,
        SensingModality::Wifi
            | SensingModality::Bluetooth
            | SensingModality::Cellular
            | SensingModality::Sdr
            | SensingModality::Radar
            | SensingModality::VisibleCamera
            | SensingModality::NearInfraredCamera
            | SensingModality::ThermalCamera
            | SensingModality::FieldInstrument
            | SensingModality::Nfc
    )
}
