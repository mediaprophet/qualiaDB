//! Compatibility header probe, independent of loaders and inner formats.
use super::{admit_header, ExtentLimit, IndexError, IndexReadLimits};
use crate::bundle::format::{BUNDLE_HEADER_SIZE, BundleError};

/// Structurally checked header fields. Public fields remain caller-constructible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HmcHeaderProbe {
    pub version: u16,
    pub flags: u16,
    pub entry_count: u32,
    pub index_offset: u64,
    pub index_length: u64,
    pub total_length: u64,
    pub whole_file_crc32c: u32,
}

impl HmcHeaderProbe {
    /// Parse the 64-byte front header of a `.hmc` bundle without needing the rest of the file.
    pub fn parse_header_range(header_bytes: &[u8]) -> Result<Self, BundleError> {
        if header_bytes.len() < BUNDLE_HEADER_SIZE {
            return Err(BundleError::TooShort);
        }
        let mut extent = [0u8; 8];
        extent.copy_from_slice(&header_bytes[28..36]);
        let checked = admit_header(
            &header_bytes[..BUNDLE_HEADER_SIZE],
            u64::from_le_bytes(extent),
            ExtentLimit::default(),
            IndexReadLimits::default(),
        )
        .map_err(IndexError::into_bundle_error)?;
        Ok(checked.probe())
    }

    /// Byte range `(offset, length)` required to fetch the CBOR index footer.
    #[inline]
    pub fn index_footer_range(&self) -> (u64, u64) {
        (self.index_offset, self.index_length)
    }
}

