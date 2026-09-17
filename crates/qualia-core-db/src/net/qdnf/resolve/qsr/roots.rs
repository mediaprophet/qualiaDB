//! Snapshot / compiler / index root binding for QSR.
//!
//! `snapshot_root` is SHA-384 over a domain-separated transcript of generation
//! then each occupied key||value pair in insert order. An empty snapshot has a
//! defined digest; it is never [`StrongDigest::ZERO`]. Binding these roots is
//! not a completeness proof.

use super::traversal::QsrSnapshot;
use crate::crypto::network::transcript::Transcript;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Transcript domain for the source snapshot root (not the compiler or index).
pub const SNAPSHOT_ROOT_DOMAIN: &[u8] = b"qdnf:qsr:snapshot-root:v1";

/// Pinned source snapshot, compiler/profile, and published index roots.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexRoots {
    /// SHA-384 over snapshot pairs (domain separated).
    pub snapshot_root: StrongDigest,
    pub compiler_digest: StrongDigest,
    pub index_root: StrongDigest,
}

/// Bind compiler and index digests to the snapshot's independently hashed root.
pub fn bind_roots(
    snapshot: &QsrSnapshot,
    compiler_digest: StrongDigest,
    index_root: StrongDigest,
) -> Result<IndexRoots, QdnfError> {
    Ok(IndexRoots {
        snapshot_root: snapshot_root(snapshot)?,
        compiler_digest,
        index_root,
    })
}

/// Domain-separated snapshot digest. Empty maps are defined and non-zero.
pub fn snapshot_root(snapshot: &QsrSnapshot) -> Result<StrongDigest, QdnfError> {
    let mut t = Transcript::new();
    t.append(b"v", SNAPSHOT_ROOT_DOMAIN)?;
    t.append(b"generation", &snapshot.generation().0.to_be_bytes())?;
    let mut i = 0usize;
    while i < snapshot.len() {
        let (key, value) = snapshot.pair_at(i).ok_or(QdnfError::Malformed)?;
        let mut pair = [0u8; 96];
        pair[..48].copy_from_slice(&key.0);
        pair[48..].copy_from_slice(&value.0);
        t.append(b"kv", &pair)?;
        i += 1;
    }
    let digest = t.digest();
    if digest.is_zero() {
        return Err(QdnfError::CryptoFailure);
    }
    Ok(digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::types::Generation;

    #[test]
    fn empty_snapshot_root_is_defined_and_not_zero() {
        let snap = QsrSnapshot::empty(Generation(1));
        let a = snapshot_root(&snap).unwrap();
        let b = snapshot_root(&snap).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, StrongDigest::ZERO);
        let bound = bind_roots(&snap, StrongDigest::ZERO, StrongDigest::ZERO).unwrap();
        assert_eq!(bound.snapshot_root, a);
    }

    #[test]
    fn generation_and_pairs_are_bound() {
        let empty_g1 = QsrSnapshot::empty(Generation(1));
        let empty_g2 = QsrSnapshot::empty(Generation(2));
        assert_ne!(
            snapshot_root(&empty_g1).unwrap(),
            snapshot_root(&empty_g2).unwrap()
        );

        let mut with_pair = QsrSnapshot::empty(Generation(1));
        let mut key = StrongDigest::ZERO;
        let mut value = StrongDigest::ZERO;
        key.0[0] = 1;
        value.0[47] = 2;
        with_pair.insert(key, value).unwrap();
        assert_ne!(
            snapshot_root(&empty_g1).unwrap(),
            snapshot_root(&with_pair).unwrap()
        );
    }
}
