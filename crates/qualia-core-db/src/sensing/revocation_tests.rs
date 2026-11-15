use super::*;
use crate::governance::consent_contract::{ConsentScope, RevocationReceipt};
use ed25519_dalek::SigningKey;

fn grant() -> (ConsentGrant, SigningKey) {
    let signer = SigningKey::from_bytes(&[2; 32]);
    let grant = ConsentGrant::new_signed(
        [4; 32], &signer, [8; 32], ConsentScope(ConsentScope::ALL), 7, 10, 100, 1,
    );
    (grant, signer)
}

fn receipt(grant: &ConsentGrant, signer: &SigningKey, id: u8) -> RevocationReceipt {
    RevocationReceipt::new_signed([id; 32], grant, signer, 50, u64::from(id))
}

struct CheckpointAuthority { accepted: bool, valid_until: u64 }

struct FixedClock(u64);

impl TrustedUnixClock for FixedClock {
    fn now_unix(&mut self) -> Result<u64, AuthorizationError> { Ok(self.0) }
}

impl RevocationCheckpointAuthority for CheckpointAuthority {
    fn verify_current_complete(
        &self,
        _grant: &ConsentGrant,
        _checkpoint: &RevocationCheckpoint,
        _receipts: &[RevocationReceipt],
        now: u64,
    ) -> Result<(), RevocationSnapshotError> {
        if self.accepted && now <= self.valid_until {
            Ok(())
        } else {
            Err(RevocationSnapshotError::UnknownCheckpoint)
        }
    }
}

fn checkpoint(grant: &ConsentGrant, receipts: &[RevocationReceipt]) -> RevocationCheckpoint {
    RevocationCheckpoint::new(
        grant.grant_id,
        grant.compute_digest(),
        99,
        receipts.len() as u8,
        snapshot_digest(grant, receipts),
    )
}

#[test]
fn complete_verified_empty_snapshot_means_not_revoked() {
    let (grant, _) = grant();
    let receipts = [];
    let authority = CheckpointAuthority { accepted: true, valid_until: 200 };
    let verified = VerifiedRevocationSnapshot::verify(
        &grant,
        checkpoint(&grant, &receipts),
        &receipts,
        &authority,
        &mut FixedClock(100),
    )
    .unwrap();
    let digest = grant.compute_digest();
    assert_eq!(verified.is_revoked(&grant.grant_id, &digest, 150), Ok(false));
    assert_eq!(
        verified.is_revoked(&grant.grant_id, &digest, 201),
        Err(AuthorizationError::RevocationUnknown)
    );
}

#[test]
fn matching_signed_revocation_is_reported_and_other_grants_fail_unknown() {
    let (grant, signer) = grant();
    let records = [receipt(&grant, &signer, 11)];
    let authority = CheckpointAuthority { accepted: true, valid_until: 200 };
    let verified = VerifiedRevocationSnapshot::verify(
        &grant,
        checkpoint(&grant, &records),
        &records,
        &authority,
        &mut FixedClock(100),
    )
    .unwrap();
    let digest = grant.compute_digest();
    assert_eq!(verified.is_revoked(&grant.grant_id, &digest, 150), Ok(true));
    assert_eq!(
        verified.is_revoked(&[99; 32], &digest, 150),
        Err(AuthorizationError::RevocationUnknown)
    );
    assert_eq!(
        verified.is_revoked(&grant.grant_id, &[0; 32], 150),
        Err(AuthorizationError::RevocationUnknown)
    );
}

#[test]
fn absent_receipt_without_fresh_complete_checkpoint_fails_closed() {
    let (grant, _) = grant();
    let records = [];
    assert_eq!(
        VerifiedRevocationSnapshot::verify(
            &grant,
            checkpoint(&grant, &records),
            &records,
            &CheckpointAuthority { accepted: false, valid_until: 200 },
            &mut FixedClock(100),
        )
        .err(),
        Some(RevocationSnapshotError::UnknownCheckpoint)
    );
}

#[test]
fn rejects_wrong_grant_bad_signature_and_duplicate_receipt() {
    let (grant, signer) = grant();
    let valid = receipt(&grant, &signer, 12);

    let other = ConsentGrant::new_signed(
        [5; 32],
        &signer,
        [8; 32],
        ConsentScope(ConsentScope::ALL),
        7,
        10,
        100,
        2,
    );
    let wrong_records = [receipt(&other, &signer, 13)];
    assert_eq!(
        VerifiedRevocationSnapshot::verify(
            &grant,
            checkpoint(&grant, &wrong_records),
            &wrong_records,
            &CheckpointAuthority { accepted: true, valid_until: 200 },
            &mut FixedClock(100),
        )
        .err(),
        Some(RevocationSnapshotError::WrongGrant)
    );

    let mut invalid = valid.clone();
    invalid.signature[0] ^= 1;
    let invalid_records = [invalid];
    assert_eq!(
        VerifiedRevocationSnapshot::verify(
            &grant,
            checkpoint(&grant, &invalid_records),
            &invalid_records,
            &CheckpointAuthority { accepted: true, valid_until: 200 },
            &mut FixedClock(100),
        )
        .err(),
        Some(RevocationSnapshotError::InvalidReceipt)
    );

    let duplicates = [valid.clone(), valid];
    assert_eq!(
        VerifiedRevocationSnapshot::verify(
            &grant,
            checkpoint(&grant, &duplicates),
            &duplicates,
            &CheckpointAuthority { accepted: true, valid_until: 200 },
            &mut FixedClock(100),
        )
        .err(),
        Some(RevocationSnapshotError::DuplicateReceipt)
    );
}

#[test]
fn rejects_checkpoint_or_snapshot_tampering() {
    let (grant, signer) = grant();
    let records = [receipt(&grant, &signer, 20)];
    let mut wrong_checkpoint = checkpoint(&grant, &records);
    wrong_checkpoint.snapshot_digest[0] ^= 1;
    assert_eq!(
        VerifiedRevocationSnapshot::verify(
            &grant,
            wrong_checkpoint,
            &records,
            &CheckpointAuthority { accepted: true, valid_until: 200 },
            &mut FixedClock(100),
        )
        .err(),
        Some(RevocationSnapshotError::SnapshotDigestMismatch)
    );
    let mut wrong_grant = checkpoint(&grant, &records);
    wrong_grant.grant_digest[0] ^= 1;
    assert_eq!(
        VerifiedRevocationSnapshot::verify(
            &grant,
            wrong_grant,
            &records,
            &CheckpointAuthority { accepted: true, valid_until: 200 },
            &mut FixedClock(100),
        )
        .err(),
        Some(RevocationSnapshotError::CheckpointMismatch)
    );
}
