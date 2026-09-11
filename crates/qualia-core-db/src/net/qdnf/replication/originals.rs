//! Signed originals stored separately from derived projections (E06.4).
//!
//! Large objects are digested in [`STREAM_PAGE_BYTES`] pages with a running
//! SHA-384. Length mismatch is [`QdnfError::Range`]. Digest mismatch is
//! [`QdnfError::Conflict`]. This module never allocates a whole-graph buffer.
//!
//! E06.5: [`QnfExtensionAdopted`] is the evaluation result in
//! [`crate::net::qdnf::replication::qnf_eval`], not a silent adoption.

use sha2::{Digest, Sha384};

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::replication::qnf_eval::QnfExtensionAdopted;
use crate::net::qdnf::replication::stream::PAGE_BYTES;
use crate::net::qdnf::types::StrongDigest;

/// Page size for streaming original bytes. Not a dataset allocation.
pub const STREAM_PAGE_BYTES: u32 = PAGE_BYTES as u32;

/// Stored original identity: SHA-384 and exact length. Not a projection.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalObject {
    pub digest: StrongDigest,
    pub len: u32,
}

/// Digest `bytes` in [`STREAM_PAGE_BYTES`] pages. `out_buf_cap_check` is the
/// caller-owned buffer ceiling (not allocated here).
///
/// [`QdnfError::Malformed`] if `bytes` is empty. [`QdnfError::Capacity`] if
/// `bytes.len()` exceeds `out_buf_cap_check` or `u32::MAX`.
pub fn store_original(bytes: &[u8], out_buf_cap_check: u32) -> Result<OriginalObject, QdnfError> {
    if bytes.is_empty() {
        return Err(QdnfError::Malformed);
    }
    let len = u32::try_from(bytes.len()).map_err(|_| QdnfError::Range)?;
    if len > out_buf_cap_check {
        return Err(QdnfError::Capacity);
    }
    Ok(OriginalObject {
        digest: digest_pages(bytes),
        len,
    })
}

/// Projection digest is never substituted for the original identity.
#[inline]
pub fn projection_replaces_original() -> bool {
    false
}

fn digest_pages(bytes: &[u8]) -> StrongDigest {
    let mut hasher = Sha384::new();
    let mut off = 0usize;
    while off < bytes.len() {
        let end = core::cmp::min(off + PAGE_BYTES, bytes.len());
        hasher.update(&bytes[off..end]);
        off = end;
    }
    let out = hasher.finalize();
    let mut d = StrongDigest::ZERO;
    d.0.copy_from_slice(&out);
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;

    #[test]
    fn qnf_extension_flag_matches_evaluation() {
        assert!(!QnfExtensionAdopted);
        assert!(!projection_replaces_original());
        assert_eq!(
            QnfExtensionAdopted,
            crate::net::qdnf::replication::qnf_eval::EVALUATION.adopted
        );
    }

    #[test]
    fn store_original_matches_whole_sha384() {
        let bytes = [0x11u8; 80];
        let orig = store_original(&bytes, 80).unwrap();
        assert_eq!(orig.len, 80);
        assert_eq!(orig.digest, sha384(&bytes));
        assert_eq!(store_original(&bytes, 79), Err(QdnfError::Capacity));
        assert_eq!(store_original(&[], 8), Err(QdnfError::Malformed));
    }

    #[test]
    fn page_stream_digest_matches_across_page_boundary() {
        let mut bytes = [0u8; PAGE_BYTES + 32];
        let mut i = 0usize;
        while i < bytes.len() {
            bytes[i] = i as u8;
            i += 1;
        }
        let expected = sha384(&bytes);
        let orig = store_original(&bytes, bytes.len() as u32).unwrap();
        assert_eq!(orig.digest, expected);
        assert_eq!(orig.len, bytes.len() as u32);
    }
}
