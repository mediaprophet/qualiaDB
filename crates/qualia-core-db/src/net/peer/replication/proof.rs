//! Membership proofs over a checkpoint's committed ids (SVC-01.08 partial).
//!
//! Membership of listed ids is verified separately from range coverage and from
//! set completeness (`membership_implies_completeness`). The proof object
//! stores the checkpoint digest plus counts only: `count` is the committed-id
//! slice length used to build the checkpoint (never a free-standing caller
//! field) and `included` is the claimed member subset length. Merkle pages,
//! omitted-range witnesses, and neighbor metadata remain open.

use crate::crypto::network::transcript::Transcript;
use crate::net::peer::replication::checkpoint::{build_checkpoint, Checkpoint, MAX_CHECKPOINT_OPS};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

pub const MAX_PROOF_IDS: usize = 16; // == MAX_CHECKPOINT_OPS

const _: () = assert!(MAX_PROOF_IDS == MAX_CHECKPOINT_OPS);

const MEMBERSHIP_DOMAIN: &[u8] = b"qdnf:sync:membership:v1-pq";

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MembershipProof {
    pub checkpoint_digest: StrongDigest,
    /// Must equal `committed_ids.len()` used to build; caller cannot forge independently.
    pub count: u16,
    /// Number of claimed member ids in this proof (subset).
    pub included: u16,
}

/// Membership of the listed ids does not imply the checkpoint range is complete.
pub fn membership_implies_range_coverage() -> bool {
    false
}

/// Proofs do not carry private neighboring operation metadata.
pub fn private_neighbor_metadata_in_proof() -> bool {
    false
}

/// Build a proof that `members` are a subset of `committed_ids` used to build `cp`.
///
/// [`QdnfError::Capacity`] if `members.len() > MAX_PROOF_IDS` or
/// `committed_ids.len() > MAX_CHECKPOINT_OPS`.
/// [`QdnfError::Conflict`] if any member id appears more than once.
/// [`QdnfError::Malformed`] if any member is not in `committed_ids`, or if `cp`
/// was not built from `committed_ids`.
pub fn prove_membership(
    cp: &Checkpoint,
    committed_ids: &[StrongDigest],
    members: &[StrongDigest],
) -> Result<MembershipProof, QdnfError> {
    if members.len() > MAX_PROOF_IDS || committed_ids.len() > MAX_CHECKPOINT_OPS {
        return Err(QdnfError::Capacity);
    }
    if has_duplicate(members) {
        return Err(QdnfError::Conflict);
    }
    let mut i = 0usize;
    while i < members.len() {
        if !contains_id(committed_ids, &members[i]) {
            return Err(QdnfError::Malformed);
        }
        i += 1;
    }
    let rebuilt = build_checkpoint(cp.scope, committed_ids)?;
    if rebuilt != *cp {
        return Err(QdnfError::Malformed);
    }
    let count = committed_ids.len() as u16;
    let included = members.len() as u16;
    bind_membership_counts(&cp.digest, count, included)?;
    Ok(MembershipProof {
        checkpoint_digest: cp.digest,
        count,
        included,
    })
}

/// Rebuild the expected proof from `members` and `committed_ids`.
///
/// [`QdnfError::Malformed`] on mismatch, including a forged `proof.count` that
/// is not `committed_ids.len() as u16`. Capacity and duplicate outcomes follow
/// [`prove_membership`].
pub fn verify_membership(
    cp: &Checkpoint,
    committed_ids: &[StrongDigest],
    members: &[StrongDigest],
    proof: &MembershipProof,
) -> Result<(), QdnfError> {
    if proof.count != committed_ids.len() as u16 {
        return Err(QdnfError::Malformed);
    }
    let expected = prove_membership(cp, committed_ids, members)?;
    if expected != *proof {
        return Err(QdnfError::Malformed);
    }
    Ok(())
}

fn contains_id(ids: &[StrongDigest], needle: &StrongDigest) -> bool {
    let mut i = 0usize;
    while i < ids.len() {
        if ids[i] == *needle {
            return true;
        }
        i += 1;
    }
    false
}

fn has_duplicate(ids: &[StrongDigest]) -> bool {
    let mut i = 0usize;
    while i < ids.len() {
        let mut j = i + 1;
        while j < ids.len() {
            if ids[i] == ids[j] {
                return true;
            }
            j += 1;
        }
        i += 1;
    }
    false
}

/// Domain-separated count binding. The public proof stores digest + counts only;
/// extra Merkle pages stay open.
fn bind_membership_counts(
    checkpoint_digest: &StrongDigest,
    count: u16,
    included: u16,
) -> Result<(), QdnfError> {
    let mut t = Transcript::new();
    t.append(b"v", MEMBERSHIP_DOMAIN)?;
    t.append(b"cp", &checkpoint_digest.0)?;
    t.append(b"count", &count.to_be_bytes())?;
    t.append(b"included", &included.to_be_bytes())?;
    let _ = t.digest();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::replication::checkpoint::membership_implies_completeness;
    use crate::net::peer::replication::operation::{operation_id, OperationDesc, MAX_PARENTS};

    fn desc(seq: u32) -> OperationDesc {
        OperationDesc {
            author: StrongDigest([0x11; 48]),
            epoch: 1,
            sequence: seq,
            scope: 7,
            contract: StrongDigest([0x22; 48]),
            purpose: StrongDigest([0x33; 48]),
            payload_digest: StrongDigest([seq as u8; 48]),
            parents: [StrongDigest::ZERO; MAX_PARENTS],
            parent_count: 0,
        }
    }

    fn oid(seq: u32) -> StrongDigest {
        operation_id(&desc(seq))
    }

    #[test]
    fn membership_does_not_imply_coverage_and_omits_neighbors() {
        assert!(!membership_implies_range_coverage());
        assert!(!private_neighbor_metadata_in_proof());
        assert!(!membership_implies_completeness());
    }

    #[test]
    fn prove_subset_of_three_id_checkpoint_and_verify() {
        let committed = [oid(1), oid(2), oid(3)];
        let cp = build_checkpoint(7, &committed).unwrap();
        let members = [committed[0], committed[2]];
        let proof = prove_membership(&cp, &committed, &members).unwrap();
        assert_eq!(proof.checkpoint_digest, cp.digest);
        assert_eq!(proof.count, 3);
        assert_eq!(proof.included, 2);
        verify_membership(&cp, &committed, &members, &proof).unwrap();
    }

    #[test]
    fn member_not_in_committed_ids_is_malformed() {
        let committed = [oid(1), oid(2), oid(3)];
        let cp = build_checkpoint(7, &committed).unwrap();
        let members = [oid(9)];
        assert_eq!(
            prove_membership(&cp, &committed, &members),
            Err(QdnfError::Malformed)
        );
    }

    #[test]
    fn forged_proof_count_is_malformed_on_verify() {
        let committed = [oid(1), oid(2), oid(3)];
        let cp = build_checkpoint(7, &committed).unwrap();
        let members = [committed[1]];
        let mut proof = prove_membership(&cp, &committed, &members).unwrap();
        proof.count = 99;
        assert_eq!(
            verify_membership(&cp, &committed, &members, &proof),
            Err(QdnfError::Malformed)
        );
    }

    #[test]
    fn empty_members_on_empty_checkpoint_ok() {
        let cp = build_checkpoint(7, &[]).unwrap();
        let proof = prove_membership(&cp, &[], &[]).unwrap();
        assert_eq!(proof.count, 0);
        assert_eq!(proof.included, 0);
        assert_eq!(proof.checkpoint_digest, cp.digest);
        verify_membership(&cp, &[], &[], &proof).unwrap();
    }

    #[test]
    fn duplicate_members_is_conflict() {
        let committed = [oid(1), oid(2), oid(3)];
        let cp = build_checkpoint(7, &committed).unwrap();
        let members = [committed[0], committed[0]];
        assert_eq!(
            prove_membership(&cp, &committed, &members),
            Err(QdnfError::Conflict)
        );
    }

    #[test]
    fn seventeenth_member_is_capacity() {
        let committed = [oid(1), oid(2), oid(3)];
        let cp = build_checkpoint(7, &committed).unwrap();
        let mut members = [StrongDigest::ZERO; 17];
        let mut i = 0u32;
        while i < 17 {
            members[i as usize] = oid(i + 1);
            i += 1;
        }
        assert_eq!(
            prove_membership(&cp, &committed, &members),
            Err(QdnfError::Capacity)
        );
    }
}
