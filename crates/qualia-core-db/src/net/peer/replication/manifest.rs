//! Immutable content manifests: exact ranges, SHA-384 digests, codec and
//! decoded-size bounds (SVC-01.12 partial).
//!
//! The digest is caller-supplied SHA-384 of the exact artifact bytes; this
//! module does not hash a body. A container generation root is not a QSync
//! root. Raw Q42/QNF artifacts are disclosed only when every byte and
//! metadata is authorized. Packages remain open. This wave's decoded bound
//! is 4 MiB, not RAM-sized datasets.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Maximum exact ranges stored on one manifest.
pub const MAX_RANGES: usize = 8;
/// Decoded-size bound this wave (not a RAM-sized dataset ceiling).
pub const MAX_DECODED_BYTES: u32 = 4 * 1024 * 1024;

/// Half-open byte range `[offset, offset + len)` into the exact artifact.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteRange {
    pub offset: u64,
    pub len: u32,
}

/// Immutable content identity: digest, codec, decoded bound, exact ranges.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentManifest {
    pub digest: StrongDigest,
    pub codec: u16,
    pub decoded_len: u32,
    pub range_count: u8,
    ranges: [ByteRange; MAX_RANGES],
}

impl ContentManifest {
    /// Bind a manifest. Digest is not computed from a body here.
    ///
    /// [`QdnfError::Malformed`] if `digest` is [`StrongDigest::ZERO`] or `ranges` is empty.
    /// [`QdnfError::Range`] if `decoded_len` is 0 or greater than [`MAX_DECODED_BYTES`],
    /// or if `offset + len` overflows.
    /// [`QdnfError::Capacity`] if `ranges.len() > MAX_RANGES`.
    /// [`QdnfError::Overlap`] if any pair overlaps when ordered by `offset`
    /// (`end > next.offset`). Adjacent ranges (`end == next.offset`) are allowed.
    /// Unsorted input is compared pairwise without allocation.
    pub fn bind(
        digest: StrongDigest,
        codec: u16,
        decoded_len: u32,
        ranges: &[ByteRange],
    ) -> Result<Self, QdnfError> {
        if digest == StrongDigest::ZERO {
            return Err(QdnfError::Malformed);
        }
        if decoded_len == 0 || decoded_len > MAX_DECODED_BYTES {
            return Err(QdnfError::Range);
        }
        if ranges.is_empty() {
            return Err(QdnfError::Malformed);
        }
        if ranges.len() > MAX_RANGES {
            return Err(QdnfError::Capacity);
        }
        reject_overlaps(ranges)?;
        let mut stored = [ByteRange { offset: 0, len: 0 }; MAX_RANGES];
        let mut i = 0usize;
        while i < ranges.len() {
            stored[i] = ranges[i];
            i += 1;
        }
        Ok(Self {
            digest,
            codec,
            decoded_len,
            range_count: ranges.len() as u8,
            ranges: stored,
        })
    }

    #[inline]
    pub fn range_count(&self) -> usize {
        self.range_count as usize
    }

    /// Range at index `i`. Out of bounds → [`QdnfError::Range`].
    pub fn range_at(&self, i: usize) -> Result<ByteRange, QdnfError> {
        if i >= self.range_count as usize {
            return Err(QdnfError::Range);
        }
        Ok(self.ranges[i])
    }
}

/// A container generation root is not an authorized QSync root (SVC-01.14).
#[inline]
pub fn container_generation_is_qsync_root() -> bool {
    false
}

/// Raw Q42/QNF artifacts require every byte and metadata disclosure authorized.
#[inline]
pub fn raw_artifact_requires_full_authorization() -> bool {
    true
}

/// Disclose raw Q42/QNF artifact bytes. Incomplete authorization fails closed.
#[inline]
pub fn disclose_raw_artifact(fully_authorized: bool) -> Result<(), QdnfError> {
    if raw_artifact_requires_full_authorization() && !fully_authorized {
        return Err(QdnfError::Unauthorized);
    }
    Ok(())
}

fn checked_end(r: ByteRange) -> Result<u64, QdnfError> {
    r.offset
        .checked_add(u64::from(r.len))
        .ok_or(QdnfError::Range)
}

/// Pairwise overlap: after ordering a pair by `offset`, `end > next.offset`.
fn reject_overlaps(ranges: &[ByteRange]) -> Result<(), QdnfError> {
    let n = ranges.len();
    let mut i = 0usize;
    while i < n {
        let a = ranges[i];
        let a_end = checked_end(a)?;
        let mut j = i + 1;
        while j < n {
            let b = ranges[j];
            let b_end = checked_end(b)?;
            let overlap = if a.offset <= b.offset {
                a_end > b.offset
            } else {
                b_end > a.offset
            };
            if overlap {
                return Err(QdnfError::Overlap);
            }
            j += 1;
        }
        i += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(tag: u8) -> StrongDigest {
        let mut d = StrongDigest::ZERO;
        d.0[0] = tag;
        d
    }

    fn bind_ok(ranges: &[ByteRange]) -> ContentManifest {
        ContentManifest::bind(digest(1), 0, 64, ranges).unwrap()
    }

    #[test]
    fn zero_digest_is_malformed() {
        let ranges = [ByteRange { offset: 0, len: 8 }];
        assert_eq!(
            ContentManifest::bind(StrongDigest::ZERO, 0, 64, &ranges),
            Err(QdnfError::Malformed)
        );
    }

    #[test]
    fn decoded_len_zero_and_over_max_are_range() {
        let ranges = [ByteRange { offset: 0, len: 8 }];
        assert_eq!(
            ContentManifest::bind(digest(1), 0, 0, &ranges),
            Err(QdnfError::Range)
        );
        assert_eq!(
            ContentManifest::bind(digest(1), 0, MAX_DECODED_BYTES + 1, &ranges),
            Err(QdnfError::Range)
        );
        ContentManifest::bind(digest(1), 0, MAX_DECODED_BYTES, &ranges).unwrap();
    }

    #[test]
    fn empty_ranges_malformed_ninth_is_capacity() {
        assert_eq!(
            ContentManifest::bind(digest(1), 0, 64, &[]),
            Err(QdnfError::Malformed)
        );
        let nine = [
            ByteRange { offset: 0, len: 1 },
            ByteRange { offset: 1, len: 1 },
            ByteRange { offset: 2, len: 1 },
            ByteRange { offset: 3, len: 1 },
            ByteRange { offset: 4, len: 1 },
            ByteRange { offset: 5, len: 1 },
            ByteRange { offset: 6, len: 1 },
            ByteRange { offset: 7, len: 1 },
            ByteRange { offset: 8, len: 1 },
        ];
        assert_eq!(
            ContentManifest::bind(digest(1), 0, 64, &nine),
            Err(QdnfError::Capacity)
        );
    }

    #[test]
    fn overlapping_ranges_are_overlap() {
        let sorted = [
            ByteRange { offset: 0, len: 10 },
            ByteRange { offset: 8, len: 4 },
        ];
        assert_eq!(
            ContentManifest::bind(digest(1), 0, 64, &sorted),
            Err(QdnfError::Overlap)
        );
        let unsorted = [
            ByteRange { offset: 8, len: 4 },
            ByteRange { offset: 0, len: 10 },
        ];
        assert_eq!(
            ContentManifest::bind(digest(1), 0, 64, &unsorted),
            Err(QdnfError::Overlap)
        );
    }

    #[test]
    fn two_disjoint_ranges_bind_ok() {
        let ranges = [
            ByteRange {
                offset: 100,
                len: 16,
            },
            ByteRange { offset: 0, len: 8 },
        ];
        let m = bind_ok(&ranges);
        assert_eq!(m.digest, digest(1));
        assert_eq!(m.codec, 0);
        assert_eq!(m.decoded_len, 64);
        assert_eq!(m.range_count(), 2);
        assert_eq!(m.range_at(0).unwrap(), ranges[0]);
        assert_eq!(m.range_at(1).unwrap(), ranges[1]);
        assert_eq!(m.range_at(2), Err(QdnfError::Range));
        let adjacent = [
            ByteRange { offset: 0, len: 8 },
            ByteRange { offset: 8, len: 4 },
        ];
        bind_ok(&adjacent);
    }

    #[test]
    fn container_generation_is_not_qsync_root() {
        assert!(!container_generation_is_qsync_root());
    }

    #[test]
    fn raw_artifact_requires_full_authorization_is_true() {
        assert!(raw_artifact_requires_full_authorization());
        assert_eq!(disclose_raw_artifact(false), Err(QdnfError::Unauthorized));
        disclose_raw_artifact(true).unwrap();
    }
}
