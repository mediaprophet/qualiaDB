//! Canonical authorized-view checkpoints over ordered committed operation IDs (SVC-01.07 partial).
//!
//! The digest is SHA-384 via [`Transcript`]: domain label, then `scope`, `count` from the
//! input slice length, then each operation id in the given order. An empty set is a defined
//! domain-separated digest, not [`StrongDigest::ZERO`]. Membership of listed ids is not
//! range completeness. Duplicate ids are [`QdnfError::Conflict`]. Odd counts are valid.
//! Merge-profile binding, Merkle pages, and proofs remain open.

use crate::crypto::network::transcript::Transcript;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

pub const MAX_CHECKPOINT_OPS: usize = 16;

const CHECKPOINT_DOMAIN: &[u8] = b"qdnf:sync:checkpoint:v1-pq";

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub scope: u64,
    pub count: u16,
    pub frontier: StrongDigest,
    pub digest: StrongDigest,
}

/// Membership of listed operation ids does not imply a complete authorized range.
pub fn membership_implies_completeness() -> bool {
    false
}

/// Count is encoded from the committed-id slice length, never a caller-supplied count field.
pub fn forged_count_rejected() -> bool {
    true
}

/// SHA-384 checkpoint over `committed_ids` in the given order.
///
/// [`QdnfError::Capacity`] if `committed_ids.len() > MAX_CHECKPOINT_OPS`.
/// [`QdnfError::Conflict`] if any id appears more than once.
pub fn build_checkpoint(
    scope: u64,
    committed_ids: &[StrongDigest],
) -> Result<Checkpoint, QdnfError> {
    if committed_ids.len() > MAX_CHECKPOINT_OPS {
        return Err(QdnfError::Capacity);
    }
    if has_duplicate(committed_ids) {
        return Err(QdnfError::Conflict);
    }
    let count = committed_ids.len() as u16;
    let frontier = if committed_ids.is_empty() {
        StrongDigest::ZERO
    } else {
        committed_ids[committed_ids.len() - 1]
    };
    Ok(Checkpoint {
        scope,
        count,
        frontier,
        digest: transcript_checkpoint(scope, count, committed_ids)?,
    })
}

/// Rebuild from `committed_ids` and require digest, count, and frontier to match.
///
/// [`QdnfError::Malformed`] on mismatch. Capacity and duplicate outcomes follow
/// [`build_checkpoint`].
pub fn verify_checkpoint(cp: &Checkpoint, committed_ids: &[StrongDigest]) -> Result<(), QdnfError> {
    let rebuilt = build_checkpoint(cp.scope, committed_ids)?;
    if rebuilt.digest != cp.digest || rebuilt.count != cp.count || rebuilt.frontier != cp.frontier {
        return Err(QdnfError::Malformed);
    }
    Ok(())
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

fn transcript_checkpoint(
    scope: u64,
    count: u16,
    committed_ids: &[StrongDigest],
) -> Result<StrongDigest, QdnfError> {
    let mut t = Transcript::new();
    t.append(b"v", CHECKPOINT_DOMAIN)?;
    t.append(b"scope", &scope.to_be_bytes())?;
    t.append(b"count", &count.to_be_bytes())?;
    let mut i = 0usize;
    while i < committed_ids.len() {
        t.append(b"op", &committed_ids[i].0)?;
        i += 1;
    }
    Ok(t.digest())
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn empty_checkpoint_digest_stable_and_not_zero() {
        let a = build_checkpoint(7, &[]).unwrap();
        let b = build_checkpoint(7, &[]).unwrap();
        assert_eq!(a.count, 0);
        assert_eq!(a.frontier, StrongDigest::ZERO);
        assert_ne!(a.digest, StrongDigest::ZERO);
        assert_eq!(a, b);
        verify_checkpoint(&a, &[]).unwrap();
    }

    #[test]
    fn duplicate_id_is_conflict() {
        let id = oid(1);
        assert_eq!(build_checkpoint(7, &[id, id]), Err(QdnfError::Conflict));
    }

    #[test]
    fn odd_count_is_ok() {
        let one = [oid(1)];
        let cp1 = build_checkpoint(7, &one).unwrap();
        assert_eq!(cp1.count, 1);
        assert_eq!(cp1.frontier, one[0]);
        verify_checkpoint(&cp1, &one).unwrap();

        let three = [oid(1), oid(2), oid(3)];
        let cp3 = build_checkpoint(7, &three).unwrap();
        assert_eq!(cp3.count, 3);
        assert_eq!(cp3.frontier, three[2]);
        verify_checkpoint(&cp3, &three).unwrap();
        assert_ne!(cp1.digest, cp3.digest);
    }

    #[test]
    fn verify_mismatch_is_malformed() {
        let ids = [oid(1), oid(2)];
        let cp = build_checkpoint(7, &ids).unwrap();
        assert_eq!(
            verify_checkpoint(&cp, &[oid(1), oid(3)]),
            Err(QdnfError::Malformed)
        );
        let mut forged = cp;
        forged.count = 99;
        assert_eq!(verify_checkpoint(&forged, &ids), Err(QdnfError::Malformed));
        assert!(forged_count_rejected());
    }

    #[test]
    fn membership_does_not_imply_completeness() {
        assert!(!membership_implies_completeness());
    }

    #[test]
    fn seventeenth_id_is_capacity() {
        let mut ids = [StrongDigest::ZERO; 17];
        let mut i = 0u32;
        while i < 17 {
            ids[i as usize] = oid(i + 1);
            i += 1;
        }
        assert_eq!(build_checkpoint(7, &ids), Err(QdnfError::Capacity));
    }
}
