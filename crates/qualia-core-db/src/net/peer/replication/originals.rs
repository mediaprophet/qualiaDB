//! Signed originals stored separately from derived projections (E06.4).
//!
//! Large objects are digested in 4096-byte pages with a running SHA-384.
//! Length mismatch is [`QdnfError::Range`]. Digest mismatch is
//! [`QdnfError::Conflict`]. This module never allocates a whole-graph buffer.
//!
//! E06.5: [`QnfExtensionAdopted`] is false — Q42 envelopes remain. E06.6
//! process-kill disk tests stay honest via
//! [`crate::net::peer::replication::crash::disk_backend_crash_injected`] (false;
//! in-memory crash model, not an OS kill).

use sha2::{Digest, Sha384};

use crate::net::peer::replication::source::ScanSource;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Page size for streaming original bytes. Not a dataset allocation.
pub const STREAM_PAGE_BYTES: u32 = 4096;

/// E06.5 format decision: QNF extension is not adopted. Q42 envelopes remain.
#[allow(non_upper_case_globals)]
pub const QnfExtensionAdopted: bool = false;

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

/// Copy `source` in caller-owned pages, verify `expected_len` and `expected`.
///
/// [`QdnfError::Malformed`] if `expected` is zero. [`QdnfError::Capacity`] if
/// `page` is empty while `expected_len > 0`. [`QdnfError::Range`] if
/// `source.logical_len()` or copied bytes disagree with `expected_len`.
/// [`QdnfError::Conflict`] if the running digest disagrees with `expected`.
pub fn stream_original(
    source: &impl ScanSource,
    expected_len: u32,
    expected: StrongDigest,
    page: &mut [u8],
) -> Result<OriginalObject, QdnfError> {
    if expected == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }
    if expected_len == 0 {
        return Err(QdnfError::Range);
    }
    if page.is_empty() {
        return Err(QdnfError::Capacity);
    }
    if source.logical_len() != u64::from(expected_len) {
        return Err(QdnfError::Range);
    }
    let digest = hash_source(source, expected_len, page)?;
    if digest != expected {
        return Err(QdnfError::Conflict);
    }
    Ok(OriginalObject {
        digest,
        len: expected_len,
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
        let end = core::cmp::min(off + STREAM_PAGE_BYTES as usize, bytes.len());
        hasher.update(&bytes[off..end]);
        off = end;
    }
    finish(hasher)
}

fn hash_source(
    source: &impl ScanSource,
    expected_len: u32,
    page: &mut [u8],
) -> Result<StrongDigest, QdnfError> {
    let mut hasher = Sha384::new();
    let mut copied = 0u32;
    let cap = if page.len() > STREAM_PAGE_BYTES as usize {
        STREAM_PAGE_BYTES as usize
    } else {
        page.len()
    };
    while copied < expected_len {
        let remain = (expected_len - copied) as usize;
        let want = if remain < cap { remain } else { cap };
        let got = source.read_at(u64::from(copied), &mut page[..want])?;
        if got == 0 {
            return Err(QdnfError::Range);
        }
        hasher.update(&page[..got]);
        copied = copied.checked_add(got as u32).ok_or(QdnfError::Range)?;
        if copied > expected_len {
            return Err(QdnfError::Range);
        }
    }
    if copied != expected_len {
        return Err(QdnfError::Range);
    }
    Ok(finish(hasher))
}

fn finish(hasher: Sha384) -> StrongDigest {
    let out = hasher.finalize();
    let mut d = StrongDigest::ZERO;
    d.0.copy_from_slice(&out);
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;
    use crate::net::peer::replication::source::MemorySource;

    #[test]
    fn qnf_extension_not_adopted() {
        assert!(!QnfExtensionAdopted);
        assert!(!projection_replaces_original());
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
    fn page_stream_digest_matches_and_mismatch_is_conflict() {
        let mut bytes = [0u8; STREAM_PAGE_BYTES as usize + 32];
        let mut i = 0usize;
        while i < bytes.len() {
            bytes[i] = i as u8;
            i += 1;
        }
        let expected = sha384(&bytes);
        let orig = store_original(&bytes, bytes.len() as u32).unwrap();
        assert_eq!(orig.digest, expected);
        let src = MemorySource::from_slice(&bytes);
        let mut page = [0u8; STREAM_PAGE_BYTES as usize];
        let streamed = stream_original(&src, bytes.len() as u32, expected, &mut page).unwrap();
        assert_eq!(streamed.digest, orig.digest);
        assert_eq!(streamed.len, orig.len);
        let mut wrong = expected;
        wrong.0[0] ^= 1;
        assert_eq!(
            stream_original(&src, bytes.len() as u32, wrong, &mut page),
            Err(QdnfError::Conflict)
        );
        assert_eq!(
            stream_original(&src, 16, expected, &mut page),
            Err(QdnfError::Range)
        );
    }
}
