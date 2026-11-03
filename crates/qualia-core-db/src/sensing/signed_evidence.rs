//! Fixed-field signed envelope for caller-owned sensing evidence.
//!
//! The envelope signs a versioned, domain-separated SHA-256 commitment. A valid
//! signature authenticates the signed declaration only; trusted policy must
//! separately authorize the source/adapter, consent decision and revocation
//! state, and calibration artifact. Neither signature nor policy proves that a
//! physical measurement is scientifically true.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

use super::grounding::{EvidenceUncertainty, SensingEvidence, SensingEvidenceAuthenticator};
use super::record::{CalibrationArtifactReference, CalibrationDeclaration};

#[path = "signed_evidence_wire.rs"]
mod wire;

pub const SIGNED_SENSING_EVIDENCE_VERSION: u16 = 1;
pub const SIGNED_SENSING_EVIDENCE_WIRE_SIZE: usize = 398;
const SIGNING_DOMAIN: &[u8] = b"qualia:sensing:evidence:v1";
const PAYLOAD_DOMAIN: &[u8] = b"qualia:sensing:payload:v1";
const CALIBRATION_DOMAIN: &[u8] = b"qualia:sensing:calibration-artifact:v1";
const WIRE_MAGIC: [u8; 4] = *b"SSE1";

/// Domain-separated digest for raw payload bytes. This helper allocates no
/// memory; callers must hash the exact immutable payload that the envelope
/// references, and the evidence policy must bind that digest to its record.
pub fn digest_payload(payload: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(PAYLOAD_DOMAIN);
    hash.update((payload.len() as u64).to_le_bytes());
    hash.update(payload);
    hash.finalize().into()
}

/// Domain-separated digest for calibration artifact bytes. A policy resolver
/// must confirm that this digest names the artifact in `SensingEvidence`;
/// signing the digest alone is not calibration validation.
pub fn digest_calibration_artifact(artifact: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(CALIBRATION_DOMAIN);
    hash.update((artifact.len() as u64).to_le_bytes());
    hash.update(artifact);
    hash.finalize().into()
}

/// Capture time in an explicit monotonic clock domain. This is not UTC; the
/// grant policy must not compare it to a wall-clock expiry without an explicit
/// authenticated clock conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureTimestamp {
    clock_domain_id: u64,
    restart_epoch: u32,
    ticks_ns: u64,
    uncertainty_ns: u64,
}

impl CaptureTimestamp {
    pub const fn new(
        clock_domain_id: u64,
        restart_epoch: u32,
        ticks_ns: u64,
        uncertainty_ns: u64,
    ) -> Result<Self, SignedEvidenceError> {
        if clock_domain_id == 0 {
            return Err(SignedEvidenceError::InvalidIdentifier);
        }
        if ticks_ns.checked_add(uncertainty_ns).is_none() {
            return Err(SignedEvidenceError::InvalidTimestamp);
        }
        Ok(Self {
            clock_domain_id,
            restart_epoch,
            ticks_ns,
            uncertainty_ns,
        })
    }

    pub const fn clock_domain_id(self) -> u64 {
        self.clock_domain_id
    }

    pub const fn restart_epoch(self) -> u32 {
        self.restart_epoch
    }

    pub const fn ticks_ns(self) -> u64 {
        self.ticks_ns
    }

    pub const fn uncertainty_ns(self) -> u64 {
        self.uncertainty_ns
    }
}

/// Versioned signed evidence record. Fields are private to prevent post-signing
/// mutation outside the module; use `verify` before consuming it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignedSensingEvidence {
    version: u16,
    evidence: SensingEvidence,
    grant_id: [u8; 32],
    grant_digest: [u8; 32],
    decision_commitment: [u8; 32],
    adapter_id: u64,
    adapter_epoch: u32,
    capture_time: CaptureTimestamp,
    calibration_reference_id: [u8; 32],
    calibration_artifact_commitment: [u8; 32],
    payload_digest: [u8; 32],
    signature: [u8; 64],
}

impl SignedSensingEvidence {
    #[allow(clippy::too_many_arguments)]
    pub fn sign(
        evidence: SensingEvidence,
        grant_id: [u8; 32],
        grant_digest: [u8; 32],
        decision_commitment: [u8; 32],
        adapter_id: u64,
        adapter_epoch: u32,
        capture_time: CaptureTimestamp,
        calibration_reference_id: [u8; 32],
        calibration_artifact_commitment: [u8; 32],
        payload_digest: [u8; 32],
        signing_key: &SigningKey,
    ) -> Result<Self, SignedEvidenceError> {
        if all_zero(&grant_id)
            || all_zero(&grant_digest)
            || all_zero(&decision_commitment)
            || adapter_id == 0
        {
            return Err(SignedEvidenceError::InvalidIdentifier);
        }
        if all_zero(&payload_digest) {
            return Err(SignedEvidenceError::InvalidPayloadDigest);
        }
        let calibrated = matches!(
            evidence.calibration(),
            CalibrationDeclaration::Calibrated(_)
        );
        if calibrated != !all_zero(&calibration_artifact_commitment) {
            return Err(SignedEvidenceError::InvalidCalibrationCommitment);
        }
        if !calibration_reference_matches(&evidence, &calibration_reference_id) {
            return Err(SignedEvidenceError::InvalidCalibrationReference);
        }
        if let EvidenceUncertainty::RelativeMillionths(value) = evidence.uncertainty() {
            if value > super::grounding::UNCERTAINTY_SCALE {
                return Err(SignedEvidenceError::InvalidEvidence);
            }
        }

        let mut envelope = Self {
            version: SIGNED_SENSING_EVIDENCE_VERSION,
            evidence,
            grant_id,
            grant_digest,
            decision_commitment,
            adapter_id,
            adapter_epoch,
            capture_time,
            calibration_reference_id,
            calibration_artifact_commitment,
            payload_digest,
            signature: [0; 64],
        };
        envelope.signature = signing_key.sign(&envelope.canonical_digest()).to_bytes();
        Ok(envelope)
    }

    pub const fn version(self) -> u16 {
        self.version
    }

    pub const fn evidence(self) -> SensingEvidence {
        self.evidence
    }

    pub const fn grant_id(self) -> [u8; 32] {
        self.grant_id
    }

    pub const fn grant_digest(self) -> [u8; 32] {
        self.grant_digest
    }

    pub const fn decision_commitment(self) -> [u8; 32] {
        self.decision_commitment
    }

    pub const fn adapter_id(self) -> u64 {
        self.adapter_id
    }

    pub const fn adapter_epoch(self) -> u32 {
        self.adapter_epoch
    }

    pub const fn capture_time(self) -> CaptureTimestamp {
        self.capture_time
    }

    pub const fn calibration_artifact_commitment(self) -> [u8; 32] {
        self.calibration_artifact_commitment
    }

    pub const fn calibration_reference_id(self) -> [u8; 32] {
        self.calibration_reference_id
    }

    pub const fn payload_digest(self) -> [u8; 32] {
        self.payload_digest
    }

    pub const fn signature(self) -> [u8; 64] {
        self.signature
    }

    /// Encode the fixed v1 transport record into caller storage. Insufficient
    /// output leaves the buffer untouched. Parsing does not imply trust.
    pub fn encode_to(&self, out: &mut [u8]) -> Result<usize, SignedEvidenceError> {
        wire::encode(self, out)
    }

    /// Parse an exact v1 transport record into an unverified envelope.
    pub fn decode_from(input: &[u8]) -> Result<Self, SignedEvidenceError> {
        wire::decode(input)
    }

    /// Canonical digest with a versioned domain separator and explicit
    /// little-endian field encodings. The raw NQuin parity word is bound exactly
    /// as stored; this does not select either conflicting parity convention.
    pub fn canonical_digest(&self) -> [u8; 32] {
        let e = self.evidence;
        let q = e.assertion();
        let mut hash = Sha256::new();
        hash.update(SIGNING_DOMAIN);
        hash.update(WIRE_MAGIC);
        hash.update(self.version.to_le_bytes());
        hash.update((SIGNED_SENSING_EVIDENCE_WIRE_SIZE as u16).to_le_bytes());
        hash.update(e.citation_id().to_le_bytes());
        hash.update(e.source_id().to_le_bytes());
        hash.update(e.sample_id().to_le_bytes());
        hash.update(q.subject.to_le_bytes());
        hash.update(q.predicate.to_le_bytes());
        hash.update(q.object.to_le_bytes());
        hash.update(q.context.to_le_bytes());
        hash.update(q.metadata.to_le_bytes());
        hash.update(q.parity.to_le_bytes());
        hash.update(e.physical_quantity_id().to_le_bytes());
        hash.update(e.unit_id().to_le_bytes());
        match e.uncertainty() {
            EvidenceUncertainty::Unknown => hash.update([0]),
            EvidenceUncertainty::RelativeMillionths(value) => {
                hash.update([1]);
                hash.update(value.to_le_bytes());
            }
        }
        hash.update([calibration_tag(e.calibration())]);
        hash.update(self.calibration_reference_id);
        hash.update(self.grant_id);
        hash.update(self.grant_digest);
        hash.update(self.decision_commitment);
        hash.update(self.adapter_id.to_le_bytes());
        hash.update(self.adapter_epoch.to_le_bytes());
        hash.update(self.capture_time.clock_domain_id.to_le_bytes());
        hash.update(self.capture_time.restart_epoch.to_le_bytes());
        hash.update(self.capture_time.ticks_ns.to_le_bytes());
        hash.update(self.capture_time.uncertainty_ns.to_le_bytes());
        hash.update(self.calibration_artifact_commitment);
        hash.update(self.payload_digest);
        hash.finalize().into()
    }

    /// Verify signature and policy using a fresh reading from the trusted clock.
    /// This authenticates origin and authorized use only; it does not establish
    /// truth. Implementations must fail closed when current UTC is unavailable.
    pub fn verify(
        &self,
        evidence: &SensingEvidence,
        policy: &dyn SignedEvidencePolicy,
        clock: &dyn TrustedEvidenceClock,
    ) -> Result<(), SignedEvidenceError> {
        let now_unix_seconds = clock
            .now_unix_seconds()
            .ok_or(SignedEvidenceError::ClockUnavailable)?;
        self.verify_at(evidence, policy, now_unix_seconds)
    }

    fn verify_at(
        &self,
        evidence: &SensingEvidence,
        policy: &dyn SignedEvidencePolicy,
        now_unix_seconds: u64,
    ) -> Result<(), SignedEvidenceError> {
        if self.version != SIGNED_SENSING_EVIDENCE_VERSION || &self.evidence != evidence {
            return Err(SignedEvidenceError::EvidenceMismatch);
        }
        if !calibration_reference_matches(evidence, &self.calibration_reference_id) {
            return Err(SignedEvidenceError::InvalidCalibrationReference);
        }
        if !policy.calibration_commitment_matches(
            evidence,
            &self.calibration_reference_id,
            &self.calibration_artifact_commitment,
        ) {
            return Err(SignedEvidenceError::InvalidCalibrationCommitment);
        }
        if !policy.payload_digest_matches(evidence, &self.payload_digest) {
            return Err(SignedEvidenceError::InvalidPayloadDigest);
        }
        let key = policy
            .authorized_source_key(evidence.source_id(), self.adapter_id, self.adapter_epoch)
            .ok_or(SignedEvidenceError::UnauthorizedSource)?;
        let signature = Signature::from_bytes(&self.signature);
        key.verify_strict(&self.canonical_digest(), &signature)
            .map_err(|_| SignedEvidenceError::InvalidSignature)?;
        if !policy.grant_and_decision_valid(
            self,
            evidence,
            &self.grant_id,
            &self.grant_digest,
            &self.decision_commitment,
            now_unix_seconds,
        ) {
            return Err(SignedEvidenceError::GrantOrRevocationDenied);
        }
        Ok(())
    }
}

/// Trusted caller policy. Implementations must verify exact grant/decision
/// binding and current revocation state, source-key authorization for this
/// source/adapter/epoch tuple, and calibration reference-to-digest identity.
/// These hooks do not assert that a correctly signed sensor statement is
/// physically true.
pub trait SignedSensingEvidencePolicy {
    fn authorized_source_key(
        &self,
        source_id: u64,
        adapter_id: u64,
        adapter_epoch: u32,
    ) -> Option<VerifyingKey>;

    fn grant_and_decision_valid(
        &self,
        envelope: &SignedSensingEvidence,
        evidence: &SensingEvidence,
        grant_id: &[u8; 32],
        grant_digest: &[u8; 32],
        decision_commitment: &[u8; 32],
        now_unix_seconds: u64,
    ) -> bool;

    fn calibration_commitment_matches(
        &self,
        evidence: &SensingEvidence,
        calibration_reference_id: &[u8; 32],
        calibration_artifact_commitment: &[u8; 32],
    ) -> bool;

    /// Verify this digest against the exact immutable payload bytes referenced
    /// by the evidence. A signature over a caller-asserted digest is not enough.
    fn payload_digest_matches(&self, evidence: &SensingEvidence, payload_digest: &[u8; 32])
        -> bool;

    fn calibration_artifact_verified(
        &self,
        envelope: &SignedSensingEvidence,
        evidence: &SensingEvidence,
        now_unix_seconds: u64,
    ) -> bool;
}

/// Alias retained for the narrower verification contract named in the task.
pub use SignedSensingEvidencePolicy as SignedEvidencePolicy;

/// Trusted current UTC source used for grant expiry checks.
///
/// Implementations must return `None` when UTC cannot be established. The
/// value is sampled once for each verification call, never cached by the
/// authenticator. It is distinct from the signed monotonic capture clock.
pub trait TrustedEvidenceClock {
    fn now_unix_seconds(&self) -> Option<u64>;
}

/// Binds one signed envelope to trusted policy and samples trusted time on each
/// authentication or calibration check.
pub struct SignedEvidenceAuthenticator<'a> {
    envelope: &'a SignedSensingEvidence,
    policy: &'a dyn SignedEvidencePolicy,
    clock: &'a dyn TrustedEvidenceClock,
}

impl<'a> SignedEvidenceAuthenticator<'a> {
    pub const fn new(
        envelope: &'a SignedSensingEvidence,
        policy: &'a dyn SignedEvidencePolicy,
        clock: &'a dyn TrustedEvidenceClock,
    ) -> Self {
        Self {
            envelope,
            policy,
            clock,
        }
    }
}

impl SensingEvidenceAuthenticator for SignedEvidenceAuthenticator<'_> {
    fn authenticates(&self, evidence: &SensingEvidence) -> bool {
        self.envelope
            .verify(evidence, self.policy, self.clock)
            .is_ok()
    }

    fn calibration_verified(&self, evidence: &SensingEvidence) -> bool {
        let Some(now_unix_seconds) = self.clock.now_unix_seconds() else {
            return false;
        };
        self.envelope
            .verify_at(evidence, self.policy, now_unix_seconds)
            .is_ok()
            && matches!(
                evidence.calibration(),
                CalibrationDeclaration::Calibrated(_)
            )
            && self
                .policy
                .calibration_artifact_verified(self.envelope, evidence, now_unix_seconds)
    }
}

fn calibration_tag(calibration: CalibrationDeclaration) -> u8 {
    match calibration {
        CalibrationDeclaration::Unknown => 0,
        CalibrationDeclaration::Missing => 1,
        CalibrationDeclaration::Expired => 2,
        CalibrationDeclaration::OutOfRange => 3,
        CalibrationDeclaration::Uncalibrated => 4,
        CalibrationDeclaration::Calibrated(_) => 5,
    }
}

fn calibration_reference_matches(evidence: &SensingEvidence, reference_id: &[u8; 32]) -> bool {
    match evidence.calibration() {
        CalibrationDeclaration::Calibrated(reference) => {
            CalibrationArtifactReference::new(*reference_id) == Ok(reference)
        }
        _ => all_zero(reference_id),
    }
}

fn all_zero(bytes: &[u8; 32]) -> bool {
    bytes.iter().all(|&byte| byte == 0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignedEvidenceError {
    InvalidIdentifier,
    InvalidTimestamp,
    InvalidPayloadDigest,
    InvalidCalibrationCommitment,
    InvalidCalibrationReference,
    InvalidEvidence,
    EvidenceMismatch,
    UnauthorizedSource,
    InvalidSignature,
    GrantOrRevocationDenied,
    OutputTooSmall,
    InvalidWireLength,
    InvalidWireHeader,
    UnsupportedVersion,
    ClockUnavailable,
}
