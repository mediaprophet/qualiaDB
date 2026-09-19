//! Checked 64-bit segment addressing for operator companion packages (W2: EOS-021).
//!
//! Enforces:
//! - 64-bit segment offsets and lengths without truncation.
//! - Checked integer arithmetic on segment bounds (no overflow).
//! - WASM / 32-bit addressability safety checks (fails closed before truncating).
//! - CRC32 / FNV-1a checksum verification.
//! - Safe zero-copy borrowing views (`SegmentView`).

use std::fmt;

/// Known segment payload classifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SegmentKind {
    /// Exact retained source bytes (e.g. raw GGML Q4_K superblocks).
    SourcePayload = 1,
    /// Repacked bit-plane weight tiles.
    BitPlaneTiles = 2,
    /// Separated scales and offsets (e.g. Q4_K d, dmin, and packed subscales).
    ScaleMinPlane = 3,
    /// Precomputed activation lookup tables or partial sums.
    LookupTable = 4,
    /// Raw escape tiles for non-repacked regions.
    RawEscape = 5,
}

impl SegmentKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::SourcePayload),
            2 => Some(Self::BitPlaneTiles),
            3 => Some(Self::ScaleMinPlane),
            4 => Some(Self::LookupTable),
            5 => Some(Self::RawEscape),
            _ => None,
        }
    }
}

/// Errors occurring during segment addressing, validation, or slicing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentError {
    /// Offset + length arithmetic caused a 64-bit integer overflow.
    OffsetOverflow,
    /// Segment exceeds the total enclosing package size.
    OutOfBounds { offset: u64, length: u64, total: u64 },
    /// Segment address exceeds the target platform's pointer width (e.g. > 4GB on WASM32).
    AddressExceedsPlatformLimits { offset: u64, length: u64 },
    /// Segment checksum mismatch.
    ChecksumMismatch { expected: u32, actual: u32 },
    /// Unknown or unsupported segment kind.
    UnsupportedKind(u8),
    /// Subslice offset or length out of range.
    SubsliceOutOfBounds,
}

impl fmt::Display for SegmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OffsetOverflow => write!(f, "segment offset + length caused integer overflow"),
            Self::OutOfBounds { offset, length, total } => {
                write!(f, "segment [{offset}..{}) exceeds total package size {total}", offset + length)
            }
            Self::AddressExceedsPlatformLimits { offset, length } => {
                write!(f, "segment [{offset}..{}) exceeds platform pointer addressability", offset + length)
            }
            Self::ChecksumMismatch { expected, actual } => {
                write!(f, "segment checksum mismatch: expected {expected:#010x}, actual {actual:#010x}")
            }
            Self::UnsupportedKind(k) => write!(f, "unsupported segment kind id: {k}"),
            Self::SubsliceOutOfBounds => write!(f, "subslice range exceeds segment length"),
        }
    }
}

impl std::error::Error for SegmentError {}

/// Declared segment in the 64-bit segment table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentDescriptor {
    pub segment_id: u32,
    pub kind: SegmentKind,
    pub offset: u64,
    pub length: u64,
    pub checksum: u32,
}

impl SegmentDescriptor {
    pub fn new(segment_id: u32, kind: SegmentKind, offset: u64, length: u64, checksum: u32) -> Self {
        Self {
            segment_id,
            kind,
            offset,
            length,
            checksum,
        }
    }

    /// Checked end offset: `offset + length`.
    pub fn end_offset(&self) -> Result<u64, SegmentError> {
        self.offset
            .checked_add(self.length)
            .ok_or(SegmentError::OffsetOverflow)
    }

    /// Validate that the segment fits within `total_package_bytes`.
    pub fn check_bounds(&self, total_package_bytes: u64) -> Result<(), SegmentError> {
        let end = self.end_offset()?;
        if end > total_package_bytes {
            return Err(SegmentError::OutOfBounds {
                offset: self.offset,
                length: self.length,
                total: total_package_bytes,
            });
        }
        Ok(())
    }

    /// Verify addressability on the target architecture.
    ///
    /// On 32-bit platforms (e.g. wasm32), offsets and lengths must fit in `usize`
    /// to avoid 32-bit truncation hazards.
    pub fn check_platform_fit(&self) -> Result<(), SegmentError> {
        let max_addr = usize::MAX as u64;
        let end = self.end_offset()?;
        if self.offset > max_addr || self.length > max_addr || end > max_addr {
            return Err(SegmentError::AddressExceedsPlatformLimits {
                offset: self.offset,
                length: self.length,
            });
        }
        Ok(())
    }
}

/// Compute a standard 32-bit FNV-1a checksum of the payload bytes.
pub fn compute_segment_checksum(data: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for &byte in data {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// A borrowed slice representing a validated segment in memory.
#[derive(Debug, Clone, Copy)]
pub struct SegmentView<'a> {
    descriptor: SegmentDescriptor,
    data: &'a [u8],
}

impl<'a> SegmentView<'a> {
    /// Create a borrowed view from full package bytes and a descriptor.
    /// Validates bounds, platform fit, and optionally checksum.
    pub fn try_new(
        descriptor: SegmentDescriptor,
        package_bytes: &'a [u8],
        verify_checksum: bool,
    ) -> Result<Self, SegmentError> {
        descriptor.check_bounds(package_bytes.len() as u64)?;
        descriptor.check_platform_fit()?;

        let start = descriptor.offset as usize;
        let len = descriptor.length as usize;
        let data = &package_bytes[start..start + len];

        if verify_checksum && descriptor.checksum != 0 {
            let actual = compute_segment_checksum(data);
            if actual != descriptor.checksum {
                return Err(SegmentError::ChecksumMismatch {
                    expected: descriptor.checksum,
                    actual,
                });
            }
        }

        Ok(Self { descriptor, data })
    }

    pub fn descriptor(&self) -> &SegmentDescriptor {
        &self.descriptor
    }

    pub fn as_slice(&self) -> &'a [u8] {
        self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Safely subslice this segment with offset and length.
    pub fn subslice(&self, offset: usize, len: usize) -> Result<&'a [u8], SegmentError> {
        let end = offset.checked_add(len).ok_or(SegmentError::SubsliceOutOfBounds)?;
        if end > self.data.len() {
            return Err(SegmentError::SubsliceOutOfBounds);
        }
        Ok(&self.data[offset..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_bounds_and_checksum_ok() {
        let payload = b"hello operator segment data";
        let checksum = compute_segment_checksum(payload);
        let desc = SegmentDescriptor::new(1, SegmentKind::SourcePayload, 0, payload.len() as u64, checksum);

        assert_eq!(desc.end_offset().unwrap(), payload.len() as u64);
        assert!(desc.check_bounds(payload.len() as u64).is_ok());
        assert!(desc.check_platform_fit().is_ok());

        let view = SegmentView::try_new(desc, payload, true).expect("view should succeed");
        assert_eq!(view.as_slice(), payload);
        assert_eq!(view.subslice(6, 8).unwrap(), b"operator");
    }

    #[test]
    fn segment_out_of_bounds_rejected() {
        let payload = b"short";
        let desc = SegmentDescriptor::new(1, SegmentKind::BitPlaneTiles, 2, 10, 0);
        assert_eq!(
            desc.check_bounds(payload.len() as u64),
            Err(SegmentError::OutOfBounds {
                offset: 2,
                length: 10,
                total: 5,
            })
        );
    }

    #[test]
    fn segment_overflow_rejected() {
        let desc = SegmentDescriptor::new(1, SegmentKind::BitPlaneTiles, u64::MAX - 5, 10, 0);
        assert_eq!(desc.end_offset(), Err(SegmentError::OffsetOverflow));
    }

    #[test]
    fn segment_checksum_mismatch_rejected() {
        let payload = b"test payload";
        let desc = SegmentDescriptor::new(1, SegmentKind::ScaleMinPlane, 0, payload.len() as u64, 0x1234_5678);
        match SegmentView::try_new(desc, payload, true) {
            Err(SegmentError::ChecksumMismatch { expected, actual }) => {
                assert_eq!(expected, 0x1234_5678);
                assert_ne!(actual, 0x1234_5678);
            }
            other => panic!("expected ChecksumMismatch, got: {other:?}"),
        }
    }
}
