//! FND-03.01 compile-time size/role table (PARTIAL).
//!
//! Short hashes are lookup indexes. They never authorize delivery.

use crate::net::qdnf::types::{Generation, LinkId, OperationId, QHashIndex, StrongDigest};

/// SHA-384 / `StrongDigest` byte length. Full security identifier.
pub const STRONG_DIGEST_LEN: usize = 48;

/// Compact `QHashIndex` byte length. Lookup only.
pub const QHASH_INDEX_LEN: usize = 8;

/// Short hashes are never proofs or authority.
#[inline]
pub fn short_hash_is_authority() -> bool {
    false
}

/// True when the typed identifiers keep the FND-03.01 size table.
#[inline]
pub fn digest_sizes_match_types() -> bool {
    core::mem::size_of::<StrongDigest>() == STRONG_DIGEST_LEN
        && core::mem::size_of::<QHashIndex>() == QHASH_INDEX_LEN
        && core::mem::size_of::<Generation>() == 8
        && core::mem::size_of::<OperationId>() == 16
        && core::mem::size_of::<LinkId>() == 16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_size_table_matches_types() {
        assert!(digest_sizes_match_types());
        assert_eq!(core::mem::size_of::<StrongDigest>(), 48);
        assert_eq!(core::mem::size_of::<QHashIndex>(), 8);
        assert_eq!(core::mem::size_of::<Generation>(), 8);
        assert_eq!(core::mem::size_of::<OperationId>(), 16);
        assert_eq!(core::mem::size_of::<LinkId>(), 16);
        assert_ne!(STRONG_DIGEST_LEN, QHASH_INDEX_LEN);
    }

    #[test]
    fn short_hash_is_lookup_index_only() {
        assert!(!short_hash_is_authority());
    }
}
