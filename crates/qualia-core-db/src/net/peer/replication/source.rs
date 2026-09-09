//! Caller-buffered scan sources (E06.1).
//!
//! Production scans of large volumes go through [`ScanSource::read_at`] into a
//! caller-owned page. This module never materializes a RAM-sized dataset and
//! does not allocate (`Vec` / `String` / `Box`) on the read path.
//! [`MemorySource`] is an adapter over a caller-owned slice, not a synthetic
//! XOR fill and not a volume loader.

use crate::net::qdnf::errors::QdnfError;

/// Exact-range reader. Implementations must not allocate.
pub trait ScanSource {
    /// Copy up to `out.len()` bytes from `offset`. Return bytes copied.
    ///
    /// Offset at or beyond [`Self::logical_len`] yields `Ok(0)`.
    fn read_at(&self, offset: u64, out: &mut [u8]) -> Result<usize, QdnfError>;

    /// Logical length in bytes. Not an allocated buffer size.
    fn logical_len(&self) -> u64;
}

/// Adapter over a caller-owned byte slice (tests and already-resident pages).
///
/// Large volumes must not be fully loaded into a slice in order to scan;
/// they must implement [`ScanSource`] with ranged `read_at`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemorySource<'a> {
    bytes: &'a [u8],
}

impl<'a> MemorySource<'a> {
    /// Bind a caller-owned slice. The slice is not copied or grown.
    #[inline]
    pub const fn from_slice(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }
}

impl ScanSource for MemorySource<'_> {
    fn read_at(&self, offset: u64, out: &mut [u8]) -> Result<usize, QdnfError> {
        let len = self.bytes.len() as u64;
        if offset >= len {
            return Ok(0);
        }
        let start = offset as usize;
        let avail = self.bytes.len() - start;
        let n = if avail < out.len() { avail } else { out.len() };
        if n == 0 {
            return Ok(0);
        }
        out[..n].copy_from_slice(&self.bytes[start..start + n]);
        Ok(n)
    }

    #[inline]
    fn logical_len(&self) -> u64 {
        self.bytes.len() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_at_copies_slice_bytes() {
        let src = [0x10u8, 0x20, 0x30, 0x40];
        let mem = MemorySource::from_slice(&src);
        let mut out = [0u8; 3];
        assert_eq!(mem.read_at(1, &mut out).unwrap(), 3);
        assert_eq!(&out, &[0x20, 0x30, 0x40]);
        assert_eq!(mem.logical_len(), 4);
    }

    #[test]
    fn offset_beyond_length_is_zero() {
        let src = [1u8, 2, 3];
        let mem = MemorySource::from_slice(&src);
        let mut out = [0xffu8; 4];
        assert_eq!(mem.read_at(3, &mut out).unwrap(), 0);
        assert_eq!(mem.read_at(99, &mut out).unwrap(), 0);
        assert_eq!(out, [0xffu8; 4]);
    }

    #[test]
    fn two_equal_length_sources_yield_different_bytes() {
        let a = [1u8; 8];
        let b = [2u8; 8];
        let mut pa = [0u8; 8];
        let mut pb = [0u8; 8];
        assert_eq!(MemorySource::from_slice(&a).read_at(0, &mut pa).unwrap(), 8);
        assert_eq!(MemorySource::from_slice(&b).read_at(0, &mut pb).unwrap(), 8);
        assert_ne!(pa, pb);
        assert_eq!(pa, a);
        assert_eq!(pb, b);
    }
}
