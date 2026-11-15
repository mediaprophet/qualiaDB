use super::*;
use crate::NQuin;
use crate::sensing::grounding::{EvidenceUncertainty, SensingEvidence};
use crate::sensing::record::{CalibrationArtifactReference, CalibrationDeclaration};
use std::cell::Cell;

struct TestClock(Cell<Option<u64>>);

impl TestClock {
    fn at(now: u64) -> Self { Self(Cell::new(Some(now))) }
}

impl TrustedEvidenceClock for TestClock {
    fn now_unix_seconds(&self) -> Option<u64> { self.0.get() }
}

struct Policy {
    key: VerifyingKey,
    grant_valid: bool,
    calibration_binding_valid: bool,
    calibration_valid: bool,
    expires_at: u64,
}

impl SignedEvidencePolicy for Policy {
    fn authorized_source_key(&self, source_id: u64, adapter_id: u64, adapter_epoch: u32) -> Option<VerifyingKey> {
        if source_id == 71 && adapter_id == 3 && adapter_epoch == 4 {
            Some(self.key.clone())
        } else {
            None
        }
    }

    fn grant_and_decision_valid(
        &self,
        envelope: &SignedSensingEvidence,
        evidence: &SensingEvidence,
        grant_id: &[u8; 32],
        grant_digest: &[u8; 32],
        decision_commitment: &[u8; 32],
        now_unix_seconds: u64,
    ) -> bool {
        self.grant_valid
            && now_unix_seconds < self.expires_at
            && grant_id == &[1; 32]
            && grant_digest == &[11; 32]
            && decision_commitment == &[2; 32]
            && envelope.grant_id() == *grant_id
            && envelope.grant_digest() == *grant_digest
            && envelope.decision_commitment() == *decision_commitment
            && envelope.adapter_epoch() == 4
            && evidence.sample_id() == 81
    }

    fn calibration_commitment_matches(
        &self,
        evidence: &SensingEvidence,
        calibration_reference_id: &[u8; 32],
        commitment: &[u8; 32],
    ) -> bool {
        self.calibration_binding_valid
            && matches!(evidence.calibration(), CalibrationDeclaration::Calibrated(_))
            && calibration_reference_id == &[7; 32]
            && commitment == &[6; 32]
    }

    fn payload_digest_matches(
        &self,
        _evidence: &SensingEvidence,
        payload_digest: &[u8; 32],
    ) -> bool {
        payload_digest == &[9; 32]
    }

    fn calibration_artifact_verified(
        &self,
        _envelope: &SignedSensingEvidence,
        _evidence: &SensingEvidence,
        _now_unix_seconds: u64,
    ) -> bool {
        self.calibration_valid
    }
}

fn evidence() -> SensingEvidence {
    SensingEvidence::new(
        61,
        71,
        81,
        NQuin {
            subject: 10,
            predicate: 20,
            object: 30,
            context: 40,
            metadata: 50,
            // Preserve the stored value verbatim; this module doesn't pick a
            // winner between existing four-field and five-field parity APIs.
            parity: 60,
        },
        91,
        101,
        EvidenceUncertainty::RelativeMillionths(5_000),
        CalibrationDeclaration::Calibrated(CalibrationArtifactReference::new([7; 32]).unwrap()),
    )
    .unwrap()
}

fn signed(key: &SigningKey) -> SignedSensingEvidence {
    SignedSensingEvidence::sign(
        evidence(),
        [1; 32],
        [11; 32],
        [2; 32],
        3,
        4,
        CaptureTimestamp::new(5, 6, 7, 8).unwrap(),
        [7; 32],
        [6; 32],
        [9; 32],
        key,
    )
    .unwrap()
}

fn policy(key: &SigningKey) -> Policy {
    Policy {
        key: key.verifying_key(),
        grant_valid: true,
        calibration_binding_valid: true,
        calibration_valid: true,
        expires_at: 100,
    }
}

#[test]
fn signed_envelope_authenticates_only_with_key_and_live_grant_policy() {
    let key = SigningKey::from_bytes(&[42; 32]);
    let envelope = signed(&key);
    let policy = policy(&key);
    let clock = TestClock::at(99);
    assert_eq!(envelope.version(), SIGNED_SENSING_EVIDENCE_VERSION);
    assert!(envelope
        .verify(&evidence(), &policy, &clock)
        .is_ok());
    let authenticator = SignedEvidenceAuthenticator::new(&envelope, &policy, &clock);
    assert!(authenticator.authenticates(&evidence()));
    assert!(authenticator.calibration_verified(&evidence()));

    let mut wire = [0u8; SIGNED_SENSING_EVIDENCE_WIRE_SIZE];
    assert_eq!(envelope.encode_to(&mut wire), Ok(SIGNED_SENSING_EVIDENCE_WIRE_SIZE));
    let decoded = SignedSensingEvidence::decode_from(&wire).unwrap();
    assert_eq!(decoded, envelope);
    assert!(decoded.verify(&evidence(), &policy, &clock).is_ok());
}

#[test]
fn tampered_payload_digest_breaks_signature() {
    let key = SigningKey::from_bytes(&[42; 32]);
    let mut envelope = signed(&key);
    envelope.payload_digest[0] ^= 1;
    let policy = policy(&key);
    let clock = TestClock::at(99);
    assert_eq!(
        envelope.verify(&evidence(), &policy, &clock),
        Err(SignedEvidenceError::InvalidPayloadDigest)
    );
}

#[test]
fn exact_grant_digest_is_cryptographically_bound() {
    let key = SigningKey::from_bytes(&[42; 32]);
    let mut envelope = signed(&key);
    envelope.grant_digest[0] ^= 1;
    let policy = policy(&key);
    let clock = TestClock::at(99);
    assert_eq!(
        envelope.verify(&evidence(), &policy, &clock),
        Err(SignedEvidenceError::InvalidSignature)
    );
}

#[test]
fn unsigned_mismatch_expiry_and_revocation_fail_closed() {
    let key = SigningKey::from_bytes(&[42; 32]);
    let envelope = signed(&key);
    let mut policy = policy(&key);
    let clock = TestClock::at(99);

    let other = SensingEvidence::new(
        62,
        71,
        81,
        evidence().assertion(),
        91,
        101,
        EvidenceUncertainty::RelativeMillionths(5_000),
        evidence().calibration(),
    )
    .unwrap();
    assert_eq!(
        envelope.verify(&other, &policy, &clock),
        Err(SignedEvidenceError::EvidenceMismatch)
    );

    policy.grant_valid = false; // models revoked or invalid grant decision
    assert_eq!(
        envelope.verify(&evidence(), &policy, &clock),
        Err(SignedEvidenceError::GrantOrRevocationDenied)
    );
    policy.grant_valid = true;
    assert_eq!(
        envelope.verify(&evidence(), &policy, &TestClock::at(100)),
        Err(SignedEvidenceError::GrantOrRevocationDenied)
    );
}

#[test]
fn signature_does_not_claim_calibration_verification_or_physical_truth() {
    let key = SigningKey::from_bytes(&[42; 32]);
    let envelope = signed(&key);
    let mut policy = policy(&key);
    policy.calibration_valid = false;
    let clock = TestClock::at(99);
    let authenticator = SignedEvidenceAuthenticator::new(&envelope, &policy, &clock);
    assert!(authenticator.authenticates(&evidence()));
    assert!(!authenticator.calibration_verified(&evidence()));
}

#[test]
fn authenticator_reads_fresh_time_for_each_check_and_fails_closed_without_time() {
    let key = SigningKey::from_bytes(&[42; 32]);
    let envelope = signed(&key);
    let policy = policy(&key);
    let clock = TestClock::at(99);
    let authenticator = SignedEvidenceAuthenticator::new(&envelope, &policy, &clock);

    assert!(authenticator.authenticates(&evidence()));
    assert!(authenticator.calibration_verified(&evidence()));
    clock.0.set(Some(100));
    assert!(!authenticator.authenticates(&evidence()));
    assert!(!authenticator.calibration_verified(&evidence()));

    clock.0.set(None);
    assert!(!authenticator.authenticates(&evidence()));
    assert!(!authenticator.calibration_verified(&evidence()));
}

#[test]
fn source_key_authorization_is_scoped_to_adapter_epoch() {
    let key = SigningKey::from_bytes(&[42; 32]);
    let policy = policy(&key);
    assert!(policy.authorized_source_key(71, 3, 4).is_some());
    assert!(policy.authorized_source_key(71, 3, 3).is_none());
}

#[test]
fn invalid_clock_and_missing_commitments_are_rejected() {
    assert_eq!(
        CaptureTimestamp::new(0, 0, 1, 0),
        Err(SignedEvidenceError::InvalidIdentifier)
    );
    assert_eq!(
        CaptureTimestamp::new(1, 0, u64::MAX, 1),
        Err(SignedEvidenceError::InvalidTimestamp)
    );
    let key = SigningKey::from_bytes(&[42; 32]);
    assert_eq!(
        SignedSensingEvidence::sign(
            evidence(),
            [0; 32],
            [11; 32],
            [2; 32],
            3,
            4,
            CaptureTimestamp::new(5, 6, 7, 8).unwrap(),
            [7; 32],
            [6; 32],
            [9; 32],
            &key,
        ),
        Err(SignedEvidenceError::InvalidIdentifier)
    );
}

#[test]
fn transport_parse_is_untrusted_and_buffer_capacity_is_transactional() {
    let key = SigningKey::from_bytes(&[42; 32]);
    let envelope = signed(&key);
    let mut too_small = [0xA5u8; SIGNED_SENSING_EVIDENCE_WIRE_SIZE - 1];
    let before = too_small;
    assert_eq!(
        envelope.encode_to(&mut too_small),
        Err(SignedEvidenceError::OutputTooSmall)
    );
    assert_eq!(too_small, before);

    let mut wire = [0u8; SIGNED_SENSING_EVIDENCE_WIRE_SIZE];
    envelope.encode_to(&mut wire).unwrap();
    wire[SIGNED_SENSING_EVIDENCE_WIRE_SIZE - 1] ^= 1;
    let parsed = SignedSensingEvidence::decode_from(&wire).unwrap();
    let policy = policy(&key);
    let clock = TestClock::at(99);
    assert_eq!(
        parsed.verify(&evidence(), &policy, &clock),
        Err(SignedEvidenceError::InvalidSignature)
    );
}
