//! Header admission never casts remote offsets into resident address sizes.
use super::{ExtentLimit, IndexError, IndexReadLimits};
use crate::bundle::format::{BUNDLE_HEADER_SIZE, BUNDLE_MAGIC, BUNDLE_VERSION};
use super::HmcHeaderProbe;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckedHeader {
    probe: HmcHeaderProbe,
}
impl CheckedHeader {
    /// Values only. Original reserved bytes are not represented in a public probe.
    pub(crate) fn from_probe(
        probe: HmcHeaderProbe,
        expected_extent: u64,
        extent_limit: ExtentLimit,
        limits: IndexReadLimits,
    ) -> Result<Self, IndexError> {
        if probe.version != BUNDLE_VERSION {
            return Err(IndexError::UnsupportedVersion(probe.version));
        }
        if probe.flags & !1 != 0 {
            return Err(IndexError::Flags);
        }
        if probe.total_length < BUNDLE_HEADER_SIZE as u64 || probe.total_length != expected_extent {
            return Err(IndexError::Extent);
        }
        if probe.total_length > extent_limit.max_logical_bytes {
            return Err(IndexError::ExtentLimit);
        }
        let end = probe
            .index_offset
            .checked_add(probe.index_length)
            .ok_or(IndexError::Overflow)?;
        if probe.index_offset < BUNDLE_HEADER_SIZE as u64 || end != probe.total_length {
            return Err(IndexError::Footer);
        }
        let h = Self { probe };
        if probe.index_offset % h.alignment() != 0 {
            return Err(IndexError::Alignment);
        }
        h.check_limits(limits)?;
        Ok(h)
    }
    pub(crate) fn check_limits(&self, limits: IndexReadLimits) -> Result<(), IndexError> {
        if self.probe.index_length
            > u64::try_from(limits.index_bytes).map_err(|_| IndexError::Overflow)?
        {
            return Err(IndexError::EncodedLimit);
        }
        if u64::from(self.probe.entry_count)
            > u64::try_from(limits.entries).map_err(|_| IndexError::Overflow)?
        {
            return Err(IndexError::Count);
        }
        Ok(())
    }
    pub const fn probe(&self) -> HmcHeaderProbe {
        self.probe
    }
    pub const fn entry_count(&self) -> u32 {
        self.probe.entry_count
    }
    pub const fn index_offset(&self) -> u64 {
        self.probe.index_offset
    }
    pub const fn index_length(&self) -> u64 {
        self.probe.index_length
    }
    pub const fn total_length(&self) -> u64 {
        self.probe.total_length
    }
    pub const fn crc32c(&self) -> u32 {
        self.probe.whole_file_crc32c
    }
    pub const fn alignment(&self) -> u64 {
        if self.probe.flags == 1 { 4096 } else { 64 }
    }
}

pub fn admit_header(
    bytes: &[u8],
    expected_extent: u64,
    extent_limit: ExtentLimit,
    limits: IndexReadLimits,
) -> Result<CheckedHeader, IndexError> {
    if bytes.len() != BUNDLE_HEADER_SIZE {
        return Err(IndexError::HeaderLength);
    }
    if bytes[..4] != BUNDLE_MAGIC {
        return Err(IndexError::BadMagic);
    }
    if bytes[40..64].iter().any(|b| *b != 0) {
        return Err(IndexError::Reserved);
    }
    let u16_at = |n: usize| u16::from_le_bytes([bytes[n], bytes[n + 1]]);
    let u32_at =
        |n: usize| u32::from_le_bytes([bytes[n], bytes[n + 1], bytes[n + 2], bytes[n + 3]]);
    let u64_at = |n: usize| {
        u64::from_le_bytes([
            bytes[n],
            bytes[n + 1],
            bytes[n + 2],
            bytes[n + 3],
            bytes[n + 4],
            bytes[n + 5],
            bytes[n + 6],
            bytes[n + 7],
        ])
    };
    CheckedHeader::from_probe(
        HmcHeaderProbe {
            version: u16_at(4),
            flags: u16_at(6),
            entry_count: u32_at(8),
            index_offset: u64_at(12),
            index_length: u64_at(20),
            total_length: u64_at(28),
            whole_file_crc32c: u32_at(36),
        },
        expected_extent,
        extent_limit,
        limits,
    )
}
