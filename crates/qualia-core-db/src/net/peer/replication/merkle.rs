//! Bounded Merkle membership and range witnesses (E11.2).
//!
//! Membership reuses [`proof`]. Range coverage is an explicit
//! `range_complete` flag defaulting to false. Manifest paging is page index
//! plus digest. Object membership is not query completeness.

use crate::crypto::network::transcript::Transcript;
use crate::net::peer::replication::checkpoint::Checkpoint;
use crate::net::peer::replication::manifest::ContentManifest;
use crate::net::peer::replication::proof::{
    membership_implies_range_coverage, prove_membership, verify_membership, MembershipProof,
};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

const PAGE_DOMAIN: &[u8] = b"qdnf:sync:manifest-page:v1-pq";

/// Membership witness plus an explicit range-completeness bit (default false).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeWitness {
    pub membership: MembershipProof,
    pub range_complete: bool,
}

/// One manifest page: index and bound digest.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestPage {
    pub index: u16,
    pub digest: StrongDigest,
}

impl RangeWitness {
    /// Range completeness defaults to false.
    pub const fn from_membership(membership: MembershipProof) -> Self {
        Self {
            membership,
            range_complete: false,
        }
    }
}

/// Bounded membership witness. Delegates to [`prove_membership`].
pub fn bounded_membership(
    cp: &Checkpoint,
    committed_ids: &[StrongDigest],
    members: &[StrongDigest],
) -> Result<MembershipProof, QdnfError> {
    prove_membership(cp, committed_ids, members)
}

/// Verify a bounded membership witness.
pub fn verify_bounded_membership(
    cp: &Checkpoint,
    committed_ids: &[StrongDigest],
    members: &[StrongDigest],
    proof: &MembershipProof,
) -> Result<(), QdnfError> {
    verify_membership(cp, committed_ids, members, proof)
}

/// Wrap a membership proof as a range witness with `range_complete = false`.
pub fn range_witness(membership: MembershipProof) -> RangeWitness {
    RangeWitness::from_membership(membership)
}

/// Bind page `index` of `manifest`. Out of range → [`QdnfError::Range`].
pub fn manifest_page(manifest: &ContentManifest, index: u16) -> Result<ManifestPage, QdnfError> {
    let _ = manifest.range_at(index as usize)?;
    Ok(ManifestPage {
        index,
        digest: page_digest(&manifest.digest, index)?,
    })
}

/// Range completeness defaults to false and is not implied by membership.
#[inline]
pub fn range_complete_default() -> bool {
    false
}

/// Object membership is not query completeness (Recipe D leftover).
#[inline]
pub fn object_membership_is_query_completeness() -> bool {
    membership_implies_range_coverage()
}

fn page_digest(manifest_digest: &StrongDigest, index: u16) -> Result<StrongDigest, QdnfError> {
    let mut t = Transcript::new();
    t.append(b"v", PAGE_DOMAIN)?;
    t.append(b"manifest", &manifest_digest.0)?;
    t.append(b"index", &index.to_be_bytes())?;
    Ok(t.digest())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::replication::checkpoint::{
        build_checkpoint, membership_implies_completeness,
    };
    use crate::net::peer::replication::manifest::ByteRange;
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

    fn digest(tag: u8) -> StrongDigest {
        let mut d = StrongDigest::ZERO;
        d.0[0] = tag;
        d
    }

    #[test]
    fn membership_witness_reuses_proof_and_range_is_not_complete() {
        let committed = [oid(1), oid(2), oid(3)];
        let cp = build_checkpoint(7, &committed).unwrap();
        let members = [committed[0]];
        let proof = bounded_membership(&cp, &committed, &members).unwrap();
        verify_bounded_membership(&cp, &committed, &members, &proof).unwrap();
        let w = range_witness(proof);
        assert!(!w.range_complete);
        assert!(!range_complete_default());
        assert!(!membership_implies_range_coverage());
        assert!(!object_membership_is_query_completeness());
        assert!(!membership_implies_completeness());
    }

    #[test]
    fn manifest_page_index_and_digest() {
        let ranges = [
            ByteRange { offset: 0, len: 8 },
            ByteRange { offset: 8, len: 8 },
        ];
        let m = ContentManifest::bind(digest(9), 0, 64, &ranges).unwrap();
        let p0 = manifest_page(&m, 0).unwrap();
        let p1 = manifest_page(&m, 1).unwrap();
        assert_eq!(p0.index, 0);
        assert_eq!(p1.index, 1);
        assert_ne!(p0.digest, p1.digest);
        assert_ne!(p0.digest, StrongDigest::ZERO);
        assert_eq!(manifest_page(&m, 2), Err(QdnfError::Range));
    }
}
