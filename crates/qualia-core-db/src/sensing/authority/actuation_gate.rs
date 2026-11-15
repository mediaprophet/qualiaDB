//! Role separation (`Measured`, `Model`, `Derived`, `Command`, `OpaquePreserved`),
//! empirical join epistemic-class filtering (Schema 134), and fail-closed simulator
//! actuation gate for QAC Schema 207 (`actuator-command`) and Schema 208 (`transmit-command`)
//! per C02-C07.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};

use crate::governance::consent_contract::ConsentGrant;
use crate::sensing::authority::certificates::{
    CalibrationError, CalibrationValidityWindow, RationalInstant,
};
use crate::sensing::authorization::{
    AuthorizationError, RevocationAuthority, TrustedUnixClock,
};
use crate::sensing::SemanticId;

/// Domain separator for BLAKE3 simulator command authority receipt digests.
pub const COMMAND_RECEIPT_DOMAIN: &[u8] = b"QUALIA-SIMULATOR-COMMAND-RECEIPT-v1\0";

/// Domain separator for Ed25519 signatures over simulator command receipts.
pub const COMMAND_SIG_DOMAIN: &[u8] = b"QUALIA-SIMULATOR-COMMAND-SIG-v1\0";

/// Maximum entries in the zero-heap simulator command anti-replay ledger.
pub const MAX_COMMAND_REPLAY_ENTRIES: usize = 64;

/// Shared Schema 4 key 1 (`Provenance.role`) enumeration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ProvenanceRole {
    Measured = 0,
    Model = 1,
    Derived = 2,
    Command = 3,
    OpaquePreserved = 4,
}

impl ProvenanceRole {
    pub const fn from_code(code: u32) -> Result<Self, ActuationGateError> {
        match code {
            0 => Ok(Self::Measured),
            1 => Ok(Self::Model),
            2 => Ok(Self::Derived),
            3 => Ok(Self::Command),
            4 => Ok(Self::OpaquePreserved),
            _ => Err(ActuationGateError::InvalidProvenanceRoleCode(code)),
        }
    }
}

/// EMF Schema 134 (`ModelEvidence.epistemic_class`) enumeration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EpistemicClass {
    Empirical,
    ModelEstimate,
    Hypothesis,
    Counterfactual,
    Fictional,
}

impl EpistemicClass {
    pub fn from_wire_str(s: &str) -> Result<Self, ActuationGateError> {
        match s {
            "empirical" => Ok(Self::Empirical),
            "model_estimate" => Ok(Self::ModelEstimate),
            "hypothesis" => Ok(Self::Hypothesis),
            "counterfactual" => Ok(Self::Counterfactual),
            "fictional" => Ok(Self::Fictional),
            _ => Err(ActuationGateError::UnknownEpistemicClass),
        }
    }

    pub const fn as_wire_str(self) -> &'static str {
        match self {
            Self::Empirical => "empirical",
            Self::ModelEstimate => "model_estimate",
            Self::Hypothesis => "hypothesis",
            Self::Counterfactual => "counterfactual",
            Self::Fictional => "fictional",
        }
    }

    /// True only when this class may participate in an empirical observation join (C02-C07).
    pub const fn is_admissible_in_empirical_join(self) -> bool {
        matches!(self, Self::Empirical)
    }
}

/// QAC Schema 207 (`Haptic.role`) enumeration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HapticSchema207Role {
    Measurement,
    ReceptorModel,
    ActuatorCommand,
}

impl HapticSchema207Role {
    pub fn from_wire_str(s: &str) -> Result<Self, ActuationGateError> {
        match s {
            "measurement" => Ok(Self::Measurement),
            "receptor-model" => Ok(Self::ReceptorModel),
            "actuator-command" => Ok(Self::ActuatorCommand),
            _ => Err(ActuationGateError::UnknownDomainRole),
        }
    }
}

/// QAC Schema 208 (`UltrasoundAcquisition.role`) enumeration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UltrasoundSchema208Role {
    RawRf,
    BasebandIq,
    BeamSummary,
    TransmitCommand,
}

impl UltrasoundSchema208Role {
    pub fn from_wire_str(s: &str) -> Result<Self, ActuationGateError> {
        match s {
            "raw-RF" => Ok(Self::RawRf),
            "baseband-IQ" => Ok(Self::BasebandIq),
            "beam-summary" => Ok(Self::BeamSummary),
            "transmit-command" => Ok(Self::TransmitCommand),
            _ => Err(ActuationGateError::UnknownDomainRole),
        }
    }
}

/// Passive observations, integrity digests, or consent/calibration identifiers that
/// C02-C07 explicitly forbids from authorizing simulator or device actuation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassiveArtifactKind {
    /// Mode 0x0106 absorbed dose (Gy) or equivalent dose (Sv).
    IonizingDoseGyOrSv,
    /// Mode 0x0107 Mechanical Index (MI) or Thermal Index (TI).
    UltrasoundMechanicalOrThermalIndex,
    /// Mode 0x0106 measured haptic force (N) or indentation (m).
    MeasuredHapticForceOrIndentation,
    /// Mode 0x0105 pitch, voicing, loudness, or perceptual audio feature.
    PerceptualPitchOrVoiceFeature,
    /// Header/directory CRC-32C, payload BLAKE3, or index commitment.
    IntegrityCrcOrBlake3Digest,
    /// Passive capture/process/store/export consent grant ID.
    PassiveConsentGrantId,
    /// Calibration certificate digest (Schema 107 / Schema 202).
    CalibrationCertificateDigest,
}

/// Unconditionally reject any attempt to treat a passive observation, metric, digest,
/// calibration receipt, or passive consent ID as actuation authorization (C02-C07).
pub const fn reject_passive_artifact_for_actuation(
    artifact: PassiveArtifactKind,
) -> Result<(), ActuationGateError> {
    Err(ActuationGateError::PassiveObservationCannotAuthorizeActuation(artifact))
}

/// Provenance descriptor (Schema 4) checked for role-specific required fields (C02-C07).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProvenanceRoleRecord {
    pub role: ProvenanceRole,
    pub source_digest: Option<[u8; 32]>,
    pub algorithm_id: SemanticId,
    pub algorithm_version: SemanticId,
    pub instrument_uri: SemanticId,
    pub authority_receipt_digest: Option<[u8; 32]>,
}

impl ProvenanceRoleRecord {
    /// Enforce C02-C07 role separation:
    /// - `Measured` requires an explicit `instrument_uri`.
    /// - `Model` and `Derived` require `algorithm_id`, `algorithm_version`, and a non-zero `source_digest`.
    /// - `OpaquePreserved` requires a non-zero `source_digest`.
    /// - `Command` requires a non-zero `authority_receipt_digest`.
    pub fn validate(&self) -> Result<(), ActuationGateError> {
        if let Some(d) = self.source_digest {
            if d == [0u8; 32] {
                return Err(ActuationGateError::InvalidIdentifier);
            }
        }
        if let Some(r) = self.authority_receipt_digest {
            if r == [0u8; 32] {
                return Err(ActuationGateError::InvalidIdentifier);
            }
        }
        match self.role {
            ProvenanceRole::Measured => {
                if self.instrument_uri.value().is_none() {
                    return Err(ActuationGateError::MissingInstrumentForMeasuredRole);
                }
            }
            ProvenanceRole::Model | ProvenanceRole::Derived => {
                if self.algorithm_id.value().is_none()
                    || self.algorithm_version.value().is_none()
                    || self.source_digest.is_none()
                {
                    return Err(ActuationGateError::MissingAlgorithmOrSourceForModelDerivedRole);
                }
            }
            ProvenanceRole::OpaquePreserved => {
                if self.source_digest.is_none() {
                    return Err(ActuationGateError::MissingSourceDigestForOpaqueRole);
                }
            }
            ProvenanceRole::Command => {
                if self.authority_receipt_digest.is_none() {
                    return Err(ActuationGateError::MissingCommandReceiptForCommandRole);
                }
            }
        }
        Ok(())
    }
}

/// Enforce C02-C07 empirical join admission across two streams:
/// - Both streams must have `ProvenanceRole::Measured` (or `Derived` from empirical measurements;
///   `Model`, `Command`, and `OpaquePreserved` are rejected from empirical joins).
/// - Both streams must have `EpistemicClass::Empirical` (`Hypothesis`, `Counterfactual`,
///   `Fictional`, and `ModelEstimate` are rejected from empirical observation joins even when
///   frequency, clock, and coordinates match).
pub fn verify_empirical_join_admission(
    left_role: ProvenanceRole,
    left_class: EpistemicClass,
    right_role: ProvenanceRole,
    right_class: EpistemicClass,
) -> Result<(), ActuationGateError> {
    for role in [left_role, right_role] {
        if !matches!(role, ProvenanceRole::Measured | ProvenanceRole::Derived) {
            return Err(ActuationGateError::RoleForbiddenInEmpiricalJoin(role));
        }
    }
    for class in [left_class, right_class] {
        if !class.is_admissible_in_empirical_join() {
            return Err(ActuationGateError::NonEmpiricalClassRejected(class));
        }
    }
    Ok(())
}

/// Simulator command target schema and domain role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum SimulatorCommandKind {
    /// QAC Schema 207 `Haptic` with `role = "actuator-command"`.
    HapticActuatorCommandSchema207 = 207,
    /// QAC Schema 208 `UltrasoundAcquisition` with `role = "transmit-command"`.
    UltrasoundTransmitCommandSchema208 = 208,
}

impl SimulatorCommandKind {
    pub const fn schema_id(self) -> u16 {
        self as u16
    }

    /// Validate that the schema-specific role matches the required command variant.
    pub const fn from_haptic_role(role: HapticSchema207Role) -> Result<Self, ActuationGateError> {
        match role {
            HapticSchema207Role::ActuatorCommand => Ok(Self::HapticActuatorCommandSchema207),
            HapticSchema207Role::Measurement | HapticSchema207Role::ReceptorModel => {
                Err(ActuationGateError::NonCommandDomainRoleRejected)
            }
        }
    }

    /// Validate that the schema-208 ultrasound role is `TransmitCommand`.
    pub const fn from_ultrasound_role(
        role: UltrasoundSchema208Role,
    ) -> Result<Self, ActuationGateError> {
        match role {
            UltrasoundSchema208Role::TransmitCommand => {
                Ok(Self::UltrasoundTransmitCommandSchema208)
            }
            UltrasoundSchema208Role::RawRf
            | UltrasoundSchema208Role::BasebandIq
            | UltrasoundSchema208Role::BeamSummary => {
                Err(ActuationGateError::NonCommandDomainRoleRejected)
            }
        }
    }
}

/// Unsigned claims committed inside a simulator command receipt (Schema 207 / Schema 208).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimulatorCommandClaims {
    pub command_id: [u8; 32],
    pub command_nonce: u64,
    pub command_kind: SimulatorCommandKind,
    pub provenance_role: ProvenanceRole,
    pub grant_id: [u8; 32],
    pub grant_digest: [u8; 32],
    pub actor_id: [u8; 32],
    pub source_id: u64,
    pub device_profile_digest: [u8; 32],
    pub contact_or_array_frame_id: u32,
    pub validity_window: CalibrationValidityWindow,
}

impl SimulatorCommandClaims {
    pub fn validate(&self) -> Result<(), ActuationGateError> {
        if self.provenance_role != ProvenanceRole::Command {
            return Err(ActuationGateError::NonCommandProvenanceRoleRejected(
                self.provenance_role,
            ));
        }
        if self.command_id == [0u8; 32]
            || self.command_nonce == 0
            || self.grant_id == [0u8; 32]
            || self.grant_digest == [0u8; 32]
            || self.actor_id == [0u8; 32]
            || self.source_id == 0
            || self.device_profile_digest == [0u8; 32]
            || self.contact_or_array_frame_id == 0
        {
            return Err(ActuationGateError::InvalidIdentifier);
        }
        Ok(())
    }
}

/// Signed simulator command receipt bound to exact device profile digest, contact/array
/// coordinate frame ID, half-open command expiry window $[t_0, t_{\text{exp}})$, and grant digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignedSimulatorCommandReceipt {
    claims: SimulatorCommandClaims,
    signer_public_key: [u8; 32],
    signature: [u8; 64],
}

impl SignedSimulatorCommandReceipt {
    pub fn sign(
        claims: SimulatorCommandClaims,
        signing_key: &SigningKey,
    ) -> Result<Self, ActuationGateError> {
        claims.validate()?;
        let signer_public_key = signing_key.verifying_key().to_bytes();
        let preimage = Self::signing_preimage(&claims, &signer_public_key);
        let signature = signing_key.sign(&preimage).to_bytes();
        Ok(Self {
            claims,
            signer_public_key,
            signature,
        })
    }

    pub const fn claims(&self) -> &SimulatorCommandClaims {
        &self.claims
    }

    pub const fn signer_public_key(&self) -> &[u8; 32] {
        &self.signer_public_key
    }

    pub const fn signature(&self) -> &[u8; 64] {
        &self.signature
    }

    /// Compute the 32-byte BLAKE3 `authority_receipt_digest` stored in Schema 4 key 6
    /// and Schema 207 key 5 for this command.
    pub fn receipt_digest(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(COMMAND_RECEIPT_DOMAIN);
        Self::hash_claims(&mut hasher, &self.claims);
        hasher.update(&self.signer_public_key);
        hasher.update(&self.signature);
        *hasher.finalize().as_bytes()
    }

    pub fn verify_signature(
        &self,
        trusted_command_authority_key: &[u8; 32],
    ) -> Result<(), ActuationGateError> {
        self.claims.validate()?;
        if &self.signer_public_key != trusted_command_authority_key {
            return Err(ActuationGateError::UntrustedCommandSigner);
        }
        let vk = VerifyingKey::from_bytes(&self.signer_public_key)
            .map_err(|_| ActuationGateError::InvalidSignature)?;
        let sig = Signature::from_bytes(&self.signature);
        vk.verify_strict(
            &Self::signing_preimage(&self.claims, &self.signer_public_key),
            &sig,
        )
        .map_err(|_| ActuationGateError::InvalidSignature)
    }

    fn signing_preimage(
        claims: &SimulatorCommandClaims,
        signer_public_key: &[u8; 32],
    ) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(COMMAND_SIG_DOMAIN);
        Self::hash_claims(&mut hasher, claims);
        hasher.update(signer_public_key);
        *hasher.finalize().as_bytes()
    }

    fn hash_claims(hasher: &mut blake3::Hasher, claims: &SimulatorCommandClaims) {
        hasher.update(&claims.command_id);
        hasher.update(&claims.command_nonce.to_le_bytes());
        hasher.update(&claims.command_kind.schema_id().to_le_bytes());
        hasher.update(&(claims.provenance_role as u32).to_le_bytes());
        hasher.update(&claims.grant_id);
        hasher.update(&claims.grant_digest);
        hasher.update(&claims.actor_id);
        hasher.update(&claims.source_id.to_le_bytes());
        hasher.update(&claims.device_profile_digest);
        hasher.update(&claims.contact_or_array_frame_id.to_le_bytes());
        hash_instant(hasher, claims.validity_window.valid_from());
        hash_instant(hasher, claims.validity_window.valid_until());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ConsumedCommandEntry {
    receipt_digest: [u8; 32],
    command_id: [u8; 32],
    command_nonce: u64,
}

/// Bounded zero-heap replay ledger for simulator command receipts.
#[derive(Clone, Debug)]
pub struct CommandReplayLedger {
    entries: [Option<ConsumedCommandEntry>; MAX_COMMAND_REPLAY_ENTRIES],
    len: usize,
}

impl Default for CommandReplayLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandReplayLedger {
    pub const fn new() -> Self {
        Self {
            entries: [None; MAX_COMMAND_REPLAY_ENTRIES],
            len: 0,
        }
    }

    pub fn record_consumption(
        &mut self,
        receipt: &SignedSimulatorCommandReceipt,
    ) -> Result<[u8; 32], ActuationGateError> {
        let digest = receipt.receipt_digest();
        let claims = receipt.claims();
        for slot in self.entries.iter().take(self.len).flatten() {
            if slot.receipt_digest == digest
                || slot.command_id == claims.command_id
                || slot.command_nonce == claims.command_nonce
            {
                return Err(ActuationGateError::CommandReplayDetected);
            }
        }
        if self.len >= MAX_COMMAND_REPLAY_ENTRIES {
            return Err(ActuationGateError::CommandReplayLedgerFull);
        }
        self.entries[self.len] = Some(ConsumedCommandEntry {
            receipt_digest: digest,
            command_id: claims.command_id,
            command_nonce: claims.command_nonce,
        });
        self.len += 1;
        Ok(digest)
    }
}

/// Request parameters for simulator command admission (Schema 207 / Schema 208).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimulatorCommandRequest {
    pub requested_kind: SimulatorCommandKind,
    pub descriptor_provenance_role: ProvenanceRole,
    pub stored_authority_receipt_digest: [u8; 32],
    pub device_profile_digest: [u8; 32],
    pub contact_or_array_frame_id: u32,
    pub actor_id: [u8; 32],
    pub source_id: u64,
    pub execution_instant: RationalInstant,
}

/// Non-copy single-use token proving that a simulator command passed all C02-C07 gates.
#[derive(Debug, PartialEq, Eq)]
pub struct AdmittedSimulatorCommand {
    receipt_digest: [u8; 32],
    command_kind: SimulatorCommandKind,
    device_profile_digest: [u8; 32],
    contact_or_array_frame_id: u32,
    execution_instant: RationalInstant,
}

impl AdmittedSimulatorCommand {
    pub const fn receipt_digest(&self) -> &[u8; 32] {
        &self.receipt_digest
    }
    pub const fn command_kind(&self) -> SimulatorCommandKind {
        self.command_kind
    }
    pub const fn device_profile_digest(&self) -> &[u8; 32] {
        &self.device_profile_digest
    }
    pub const fn contact_or_array_frame_id(&self) -> u32 {
        self.contact_or_array_frame_id
    }
    pub const fn execution_instant(&self) -> RationalInstant {
        self.execution_instant
    }
}

/// Admit a simulator command (Schema 207 `actuator-command` or Schema 208 `transmit-command`)
/// only when all C02-C07 conditions hold:
/// 1. `descriptor_provenance_role == ProvenanceRole::Command` (measured/model/derived/opaque roles reject).
/// 2. `receipt` is validly signed by `trusted_command_authority_key` and matches `stored_authority_receipt_digest`.
/// 3. `requested_kind`, `device_profile_digest`, `contact_or_array_frame_id`, `actor_id`, and `source_id`
///    match the signed receipt claims exactly.
/// 4. `execution_instant` lies strictly within `[valid_from, valid_until)` (rejecting at `t < valid_from`
///    and at `t == valid_until` with zero time tolerance).
/// 5. The backing `ConsentGrant` is unrevoked and non-expired at `trusted_clock.now_unix()`.
/// 6. The command receipt has not been replayed in `replay_ledger`.
pub fn admit_simulator_command(
    request: &SimulatorCommandRequest,
    receipt: &SignedSimulatorCommandReceipt,
    trusted_command_authority_key: &[u8; 32],
    grant: &ConsentGrant,
    revocations: &impl RevocationAuthority,
    trusted_clock: &mut impl TrustedUnixClock,
    replay_ledger: &mut CommandReplayLedger,
) -> Result<AdmittedSimulatorCommand, ActuationGateError> {
    if request.descriptor_provenance_role != ProvenanceRole::Command {
        return Err(ActuationGateError::NonCommandProvenanceRoleRejected(
            request.descriptor_provenance_role,
        ));
    }
    receipt.verify_signature(trusted_command_authority_key)?;
    let computed_digest = receipt.receipt_digest();
    if computed_digest != request.stored_authority_receipt_digest {
        return Err(ActuationGateError::CommandReceiptDigestMismatch);
    }

    let claims = receipt.claims();
    if claims.command_kind != request.requested_kind {
        return Err(ActuationGateError::CommandKindMismatch);
    }
    if claims.device_profile_digest != request.device_profile_digest {
        return Err(ActuationGateError::DeviceProfileDigestMismatch);
    }
    if claims.contact_or_array_frame_id != request.contact_or_array_frame_id
        || request.contact_or_array_frame_id == 0
    {
        return Err(ActuationGateError::ContactOrArrayFrameMismatch);
    }
    if claims.actor_id != request.actor_id || claims.actor_id != grant.recipient_did {
        return Err(ActuationGateError::ActorMismatch);
    }
    if claims.source_id != request.source_id {
        return Err(ActuationGateError::SourceIdMismatch);
    }
    if claims.grant_id != grant.grant_id || claims.grant_digest != grant.compute_digest() {
        return Err(ActuationGateError::GrantBindingMismatch);
    }

    // Half-open command validity window [t_0, t_exp): exact boundary t == t_exp fails closed.
    claims
        .validity_window
        .verify_contains(request.execution_instant)
        .map_err(ActuationGateError::CommandWindow)?;

    let now_unix = trusted_clock
        .now_unix()
        .map_err(|_| ActuationGateError::ClockUnavailable)?;
    if now_unix < grant.created_at {
        return Err(ActuationGateError::GrantNotYetValid);
    }
    grant
        .verify(now_unix)
        .map_err(|_| ActuationGateError::GrantExpiredOrInvalid)?;
    let is_revoked = revocations
        .is_revoked(&grant.grant_id, &claims.grant_digest, now_unix)
        .map_err(|_| ActuationGateError::RevocationCheckFailed)?;
    if is_revoked {
        return Err(ActuationGateError::GrantRevoked);
    }

    let recorded_digest = replay_ledger.record_consumption(receipt)?;
    Ok(AdmittedSimulatorCommand {
        receipt_digest: recorded_digest,
        command_kind: claims.command_kind,
        device_profile_digest: claims.device_profile_digest,
        contact_or_array_frame_id: claims.contact_or_array_frame_id,
        execution_instant: request.execution_instant,
    })
}

fn hash_instant(hasher: &mut blake3::Hasher, instant: RationalInstant) {
    hasher.update(&instant.clock_origin().value().unwrap_or(0).to_le_bytes());
    hasher.update(&instant.restart_epoch().to_le_bytes());
    hasher.update(&instant.scale().to_le_bytes());
    hasher.update(&instant.numerator().to_le_bytes());
    hasher.update(&instant.denominator().to_le_bytes());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActuationGateError {
    InvalidIdentifier,
    InvalidProvenanceRoleCode(u32),
    UnknownEpistemicClass,
    UnknownDomainRole,
    MissingInstrumentForMeasuredRole,
    MissingAlgorithmOrSourceForModelDerivedRole,
    MissingSourceDigestForOpaqueRole,
    MissingCommandReceiptForCommandRole,
    RoleForbiddenInEmpiricalJoin(ProvenanceRole),
    NonEmpiricalClassRejected(EpistemicClass),
    PassiveObservationCannotAuthorizeActuation(PassiveArtifactKind),
    NonCommandDomainRoleRejected,
    NonCommandProvenanceRoleRejected(ProvenanceRole),
    UntrustedCommandSigner,
    InvalidSignature,
    CommandReceiptDigestMismatch,
    CommandKindMismatch,
    DeviceProfileDigestMismatch,
    ContactOrArrayFrameMismatch,
    ActorMismatch,
    SourceIdMismatch,
    GrantBindingMismatch,
    CommandWindow(CalibrationError),
    ClockUnavailable,
    GrantNotYetValid,
    GrantExpiredOrInvalid,
    RevocationCheckFailed,
    GrantRevoked,
    CommandReplayDetected,
    CommandReplayLedgerFull,
    Authorization(AuthorizationError),
}
