//! Host-authorized Etalon `ObservationEnvelope` to QualiaDB EMF (`0x0105`, `0x0106`, `0x0107`)
//! and QAC (`0x0106`, `0x0107`) sensing authority bridges, exact `i128` rational calibration/clock
//! discipline, EMF/QAC I/Q carrier convention guards, and fail-closed simulator actuation gate.
//!
//!Owned by Packet `M02` (`sensing_authority_engineer` | Gates: `G-AUTHORITY`, `G-INTEGRITY`).

pub mod calibration_clock_bridge;
pub mod emf_iq_dose_bio;
pub mod qac_haptic_ultrasound;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_qac;

pub use calibration_clock_bridge::{
    apply_single_calibration_pair, blake3_domain_digest, compute_envelope_phase_uncertainty_rad,
    convert_iq_between_emf_and_qac, reject_unconjugated_iq_memcpy,
    verify_phase_coherent_envelope_join, verify_same_clock_domain, BridgeCalibrationStage,
    BridgeCalibrationWindow, BridgeIqConvention, BridgeIqSample, BridgeRationalInstant,
    CalibrationClockBridgeError,
};
pub use emf_iq_dose_bio::{
    bridge_emf_mode0105_iq_with_consent, bridge_emf_mode0106_ionizing_with_consent,
    bridge_emf_mode0107_bio_dipole_with_consent, BridgeEpistemicClass, BridgeUncertaintyKind,
    BridgedEmfMode0105Iq, BridgedEmfMode0106Ionizing, BridgedEmfMode0107BioDipole,
    ComponentTargetedUncertainty, EmfAuthorityBridgeError, EmfBridgeReceiptReplayWindow,
    EmfMode0105IqInput, EmfMode0106IonizingInput, EmfMode0107BioDipoleInput, IonizingComponentTag,
    IonizingObservationKind, SignedEmfBridgeReceipt, EMF_MODE_0105_CALIBRATED_IQ_STFT,
    EMF_MODE_0106_IONIZING_RADIATION, EMF_MODE_0107_BIOELECTRIC_DIPOLE,
};
pub use qac_haptic_ultrasound::{
    admit_qac_simulator_command, bridge_qac_mode0106_haptic_with_consent,
    bridge_qac_mode0107_ultrasound_with_consent, reject_passive_artifact_for_actuation,
    BridgeProvenanceRole, BridgedQacMode0106Haptic, BridgedQacMode0107Ultrasound,
    HapticForceConvention, PassiveObservationArtifactKind, QacAuthorityBridgeError,
    QacCommandScope, QacHapticRole, QacMode0106HapticInput, QacMode0107UltrasoundInput,
    QacReceiptReplayLedger, QacSimulatorCommandEnvelope, QacUltrasoundRole,
    SignedQacCommandReceipt, SignedQacObservationReceipt, ThermalIndexVariant,
    VerifiedQacSimulatorCommand, QAC_MODE_0106_HAPTIC_MECHANICAL, QAC_MODE_0107_ULTRASOUND_BEAM,
};
