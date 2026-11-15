//! Cryptographic authority, calibration certificate chain verification, rational clock
//! discipline, and simulator actuation gates for EMF v4 and QAC v1 streams (Packet M02).
//!
//! Enforces gates `G-AUTHORITY` and `G-INTEGRITY` under C02 (`quantities-models.md`) and
//! C03 (`wire-freeze.md`):
//! - [`custody_bridge`]: Anti-substitution binding across `authority_receipt_digest`
//!   (Schemas 4, 7, 137, 207), `original_blake3`, envelope `index_commitment`, `source_id`,
//!   `actor_id`, `purpose_hash`, and `SensingAction`, with replay protection, live revocation
//!   revalidation, sidecar attestation, and native local custody integration.
//! - [`certificates`]: Ed25519 + BLAKE3 calibration certificate chain verification (EMF Schema 107,
//!   QAC Schema 202), exact `i128` rational half-open interval validity `[valid_from, valid_until)`
//!   (C02-C03), single-application affine calibration rules (C02-C05), and explicit uncertainty
//!   target/unit/kind validation on mixed-unit records (Modes 0x0106 and 0x0107).
//! - [`clock_discipline`]: Exact rational clock origin verification, TAI/UTC leap-second table
//!   binding, sinusoidal phase uncertainty bounds $\Delta\phi_{\max} = 2\pi f u_t$ (C02-C03),
//!   and explicit EMF negative-carrier vs QAC positive-carrier I/Q conversion (`z_QAC = conj(z_EMF)`,
//!   C02-C08).
//! - [`actuation_gate`]: Strict provenance role separation (`Measured`, `Model`, `Derived`,
//!   `Command`, `OpaquePreserved`), empirical join rejection of `Hypothesis`, `Counterfactual`,
//!   and `Fictional` epistemic classes (Schema 134), passive-artifact non-actuation guards, and
//!   fail-closed simulator command admission (Schema 207 `actuator-command`, Schema 208
//!   `transmit-command`, C02-C07).

pub mod actuation_gate;
pub mod certificates;
pub mod clock_discipline;
pub mod custody_bridge;

pub use actuation_gate::{
    admit_simulator_command, reject_passive_artifact_for_actuation,
    verify_empirical_join_admission, ActuationGateError, AdmittedSimulatorCommand,
    CommandReplayLedger, EpistemicClass, HapticSchema207Role, PassiveArtifactKind, ProvenanceRole,
    ProvenanceRoleRecord, SignedSimulatorCommandReceipt, SimulatorCommandClaims,
    SimulatorCommandKind, SimulatorCommandRequest, UltrasoundSchema208Role,
    COMMAND_RECEIPT_DOMAIN, COMMAND_SIG_DOMAIN, MAX_COMMAND_REPLAY_ENTRIES,
};
pub use certificates::{
    calibrate_complex_iq_pair_once, verify_calibration_certificate_chain, CalibratedSeriesState,
    CalibrationCertificate, CalibrationError, CalibrationSchemaFamily, CalibrationValidityWindow,
    QualifiedUncertainty, RationalInstant, SignalStageRelation, UncertaintyKind,
    CALIBRATION_CERT_DOMAIN, CALIBRATION_SIG_DOMAIN, MAX_CALIBRATION_BATCH_SAMPLES,
    MAX_CALIBRATION_CHAIN_LEN,
};
pub use clock_discipline::{
    compute_phase_uncertainty_rad, convert_iq_sample_between_conventions,
    reject_unconjugated_iq_memcpy, verify_phase_coherent_join, verify_same_clock_join,
    ClockDisciplineError, ClockScale, CrossClockMapping, IqCarrierConvention, LeapSecondStep,
    LeapSecondTable, QualifiedIqSample, LEAP_SECOND_TABLE_DOMAIN, MAX_LEAP_SECOND_ENTRIES,
};
pub use custody_bridge::{
    verify_acquisition_derivation_custody, verify_acquisition_original_custody,
    verify_and_consume_stream_receipt, verify_artifact_link_digest, AuthorityBindingClaims,
    AuthorityReceiptTarget, ConsentBoundSidecarAuthority, CustodyBridgeError,
    ExpectedStreamBinding, ReceiptReplayWindow, SignedAuthorityReceipt, AUTHORITY_RECEIPT_DOMAIN,
    AUTHORITY_SIGNATURE_DOMAIN, MAX_RECEIPT_REPLAY_WINDOW,
};
#[cfg(not(target_arch = "wasm32"))]
pub use custody_bridge::LocalCustodyStoreBridge;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_custody_actuation;
