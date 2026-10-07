//! Allocation-free KTX2 container inspection shared by native and WASM callers.
//!
//! This module validates container structure and exposes borrowed byte ranges. It does not
//! decode/transcode texels, validate Data Format Descriptor format/sample semantics, or upload
//! GPU resources. It does validate the DFD's outer size and descriptor-block framing. Callers
//! must apply their own format/device capability policy before consuming data.

/// The 12-byte identifier mandated by the KTX2 container format.
pub const KTX2_IDENTIFIER: [u8; 12] = [
    0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB, 0x0D, 0x0A, 0x1A, 0x0A,
];

const HEADER_LEN: usize = 80;
const LEVEL_INDEX_ENTRY_LEN: usize = 24;

/// Structural error found while inspecting a KTX2 byte stream.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ktx2Error {
    TruncatedHeader,
    InvalidIdentifier,
    UnsupportedDimensions,
    UnsupportedFaceCount,
    UnsupportedLevelCount,
    UnsupportedTypeSize,
    UnsupportedSupercompression,
    InvalidSectionPair,
    InvalidSectionAlignment,
    SectionOutOfBounds,
    SectionOverlap,
    LevelIndexOverflow,
    TruncatedLevelIndex,
    InvalidLevelRange,
    LevelRangeOverlap,
    UncompressedLengthMismatch,
    InvalidDfd,
    InvalidSupercompressionParameters,
    InvalidSectionOrder,
    InvalidPadding,
    InvalidLevelAlignment,
    PlatformAddressOverflow,
}

/// A borrowed KTX2 container. Every exposed section has passed checked bounds and overlap tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ktx2Document<'a> {
    bytes: &'a [u8],
    pub vk_format: u32,
    pub type_size: u32,
    pub pixel_width: u32,
    /// Zero means a 1D texture, per KTX2.
    pub pixel_height: u32,
    /// Zero means a 1D/2D texture, per KTX2.
    pub pixel_depth: u32,
    /// Zero in the header means a non-array texture; `layer_count()` reports its effective count.
    pub layer_count_field: u32,
    pub face_count: u32,
    /// Zero in the header means one level with no mip chain; `level_count()` is the effective count.
    pub level_count_field: u32,
    pub supercompression_scheme: u32,
    dfd: Range,
    kvd: Range,
    sgd: Range,
    level_index_start: usize,
    level_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Range {
    start: usize,
    end: usize,
}

impl Range {
    const EMPTY: Self = Self { start: 0, end: 0 };

    fn slice<'a>(self, bytes: &'a [u8]) -> &'a [u8] {
        &bytes[self.start..self.end]
    }

    fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

/// One level-index record and its compressed/encoded byte range.
#[derive(Clone, Copy, Debug)]
pub struct Ktx2Level<'a> {
    /// Mip index, where zero is the base level.
    pub index: u32,
    /// Encoded bytes for this level (compressed when the container uses supercompression).
    pub bytes: &'a [u8],
    pub uncompressed_byte_length: u64,
}

impl<'a> Ktx2Document<'a> {
    /// Validate a KTX2 header, metadata ranges, and level index without allocating.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Ktx2Error> {
        if bytes.len() < HEADER_LEN {
            return Err(Ktx2Error::TruncatedHeader);
        }
        if bytes[..12] != KTX2_IDENTIFIER {
            return Err(Ktx2Error::InvalidIdentifier);
        }

        let vk_format = u32_at(bytes, 12);
        let type_size = u32_at(bytes, 16);
        let pixel_width = u32_at(bytes, 20);
        let pixel_height = u32_at(bytes, 24);
        let pixel_depth = u32_at(bytes, 28);
        let layer_count_field = u32_at(bytes, 32);
        let face_count = u32_at(bytes, 36);
        let level_count_field = u32_at(bytes, 40);
        let supercompression_scheme = u32_at(bytes, 44);
        if pixel_width == 0 || (pixel_height == 0 && pixel_depth != 0) {
            return Err(Ktx2Error::UnsupportedDimensions);
        }
        if !matches!(face_count, 1 | 6)
            || (face_count == 6
                && (pixel_height == 0 || pixel_depth != 0 || pixel_width != pixel_height))
        {
            return Err(Ktx2Error::UnsupportedFaceCount);
        }
        if !matches!(type_size, 1 | 2 | 4) {
            return Err(Ktx2Error::UnsupportedTypeSize);
        }
        // This parser currently supports the standard scheme IDs only. Registered vendor
        // schemes (for example 0x10000) need scheme-specific structural validators.
        if supercompression_scheme > 3 {
            return Err(Ktx2Error::UnsupportedSupercompression);
        }

        let effective_levels = if level_count_field == 0 {
            1
        } else {
            level_count_field
        };
        let max_dimension = pixel_width.max(pixel_height).max(pixel_depth);
        let max_levels = 32 - max_dimension.leading_zeros();
        if effective_levels > max_levels {
            return Err(Ktx2Error::UnsupportedLevelCount);
        }
        if vk_format == 0 && type_size != 1 {
            return Err(Ktx2Error::UnsupportedTypeSize);
        }

        let level_count =
            usize::try_from(effective_levels).map_err(|_| Ktx2Error::LevelIndexOverflow)?;
        let index_bytes = level_count
            .checked_mul(LEVEL_INDEX_ENTRY_LEN)
            .ok_or(Ktx2Error::LevelIndexOverflow)?;
        let index_end = HEADER_LEN
            .checked_add(index_bytes)
            .ok_or(Ktx2Error::LevelIndexOverflow)?;
        if index_end > bytes.len() {
            return Err(Ktx2Error::TruncatedLevelIndex);
        }

        let dfd = read_section(bytes, 48, 52, 4, true)?;
        let kvd = read_section(bytes, 56, 60, 4, false)?;
        let sgd = read_section(bytes, 64, 72, 8, false)?;
        validate_dfd(bytes, dfd)?;
        validate_metadata_order_and_padding(bytes, index_end, dfd, kvd, sgd)?;

        match supercompression_scheme {
            0 | 2 | 3 if sgd.end != sgd.start => {
                return Err(Ktx2Error::InvalidSupercompressionParameters);
            }
            1 if vk_format != 0 || sgd.end == sgd.start => {
                return Err(Ktx2Error::InvalidSupercompressionParameters);
            }
            _ => {}
        }
        let fixed = Range {
            start: 0,
            end: index_end,
        };
        for section in [dfd, kvd, sgd] {
            if section.end > section.start && section.overlaps(fixed) {
                return Err(Ktx2Error::SectionOverlap);
            }
        }
        if dfd.overlaps(kvd) || dfd.overlaps(sgd) || kvd.overlaps(sgd) {
            return Err(Ktx2Error::SectionOverlap);
        }

        let doc = Self {
            bytes,
            vk_format,
            type_size,
            pixel_width,
            pixel_height,
            pixel_depth,
            layer_count_field,
            face_count,
            level_count_field,
            supercompression_scheme,
            dfd,
            kvd,
            sgd,
            level_index_start: HEADER_LEN,
            level_count,
        };

        // Level ranges may appear in any file order, but may not overlap one another or metadata.
        for i in 0..level_count {
            let current = doc.level_range(i)?;
            if current.start == current.end || current.start < index_end {
                return Err(Ktx2Error::InvalidLevelRange);
            }
            if [dfd, kvd, sgd]
                .iter()
                .any(|section| current.overlaps(*section))
            {
                return Err(Ktx2Error::SectionOverlap);
            }
            let metadata_end = sgd.end.max(kvd.end).max(dfd.end);
            if current.start < metadata_end {
                return Err(Ktx2Error::InvalidSectionOrder);
            }
            if supercompression_scheme == 0 && current.start % 4 != 0 {
                return Err(Ktx2Error::InvalidLevelAlignment);
            }
            let uncompressed = doc.level_uncompressed_len(i);
            if supercompression_scheme == 0 && uncompressed != (current.end - current.start) as u64
            {
                return Err(Ktx2Error::UncompressedLengthMismatch);
            }
            if supercompression_scheme == 1 && uncompressed != 0 {
                return Err(Ktx2Error::InvalidSupercompressionParameters);
            }
            for j in 0..i {
                if current.overlaps(doc.level_range(j)?) {
                    return Err(Ktx2Error::LevelRangeOverlap);
                }
            }
        }
        validate_level_padding(bytes, &doc, dfd, kvd, sgd)?;
        Ok(doc)
    }

    /// Effective array layer count; non-array textures have one layer.
    pub const fn layer_count(&self) -> u32 {
        if self.layer_count_field == 0 {
            1
        } else {
            self.layer_count_field
        }
    }

    /// Effective number of indexed levels; a zero header field represents a single base level.
    pub const fn level_count(&self) -> usize {
        self.level_count
    }

    /// Borrow the Data Format Descriptor bytes. The descriptor's internal contents are opaque.
    pub fn data_format_descriptor(&self) -> &'a [u8] {
        self.dfd.slice(self.bytes)
    }

    /// Borrow the Key/Value Data bytes. Entries are intentionally not decoded here.
    pub fn key_value_data(&self) -> &'a [u8] {
        self.kvd.slice(self.bytes)
    }

    /// Borrow the Supercompression Global Data bytes, if present.
    pub fn supercompression_global_data(&self) -> &'a [u8] {
        self.sgd.slice(self.bytes)
    }

    /// Borrow one encoded level and report its declared uncompressed length.
    pub fn level(&self, index: usize) -> Option<Ktx2Level<'a>> {
        if index >= self.level_count {
            return None;
        }
        let range = self.level_range(index).ok()?;
        Some(Ktx2Level {
            index: index as u32,
            bytes: &self.bytes[range.start..range.end],
            uncompressed_byte_length: self.level_uncompressed_len(index),
        })
    }

    fn level_range(&self, index: usize) -> Result<Range, Ktx2Error> {
        let entry = self
            .level_index_start
            .checked_add(
                index
                    .checked_mul(LEVEL_INDEX_ENTRY_LEN)
                    .ok_or(Ktx2Error::LevelIndexOverflow)?,
            )
            .ok_or(Ktx2Error::LevelIndexOverflow)?;
        let offset = u64_at(self.bytes, entry);
        let length = u64_at(self.bytes, entry + 8);
        if length == 0 {
            return Err(Ktx2Error::InvalidLevelRange);
        }
        let end = offset
            .checked_add(length)
            .ok_or(Ktx2Error::InvalidLevelRange)?;
        let start = usize::try_from(offset).map_err(|_| Ktx2Error::PlatformAddressOverflow)?;
        let end = usize::try_from(end).map_err(|_| Ktx2Error::PlatformAddressOverflow)?;
        if start > end || end > self.bytes.len() {
            return Err(Ktx2Error::InvalidLevelRange);
        }
        Ok(Range { start, end })
    }

    fn level_uncompressed_len(&self, index: usize) -> u64 {
        let entry = self.level_index_start + index * LEVEL_INDEX_ENTRY_LEN;
        u64_at(self.bytes, entry + 16)
    }
}

/// Validate the DFD envelope and descriptor-block extents without interpreting its format data.
fn validate_dfd(bytes: &[u8], dfd: Range) -> Result<(), Ktx2Error> {
    const DFD_HEADER_BYTES: usize = 4;
    const DFD_BLOCK_HEADER_BYTES: usize = 8;
    const DFD_BLOCK_MIN_BYTES: usize = 24;

    if dfd.end - dfd.start < DFD_HEADER_BYTES + DFD_BLOCK_MIN_BYTES {
        return Err(Ktx2Error::InvalidDfd);
    }
    let total_size = u32_at(bytes, dfd.start) as usize;
    if total_size != dfd.end - dfd.start {
        return Err(Ktx2Error::InvalidDfd);
    }
    let mut cursor = dfd.start + DFD_HEADER_BYTES;
    while cursor < dfd.end {
        if dfd.end - cursor < DFD_BLOCK_HEADER_BYTES {
            return Err(Ktx2Error::InvalidDfd);
        }
        // The block size is the high 16 bits of the second UInt32 in the block header.
        let block_size = (u32_at(bytes, cursor + 4) >> 16) as usize;
        if block_size < DFD_BLOCK_MIN_BYTES || block_size % 4 != 0 || block_size > dfd.end - cursor
        {
            return Err(Ktx2Error::InvalidDfd);
        }
        cursor += block_size;
    }
    if cursor != dfd.end {
        return Err(Ktx2Error::InvalidDfd);
    }
    Ok(())
}

/// Enforce KTX2's DFD → KVD → SGD ordering and all metadata alignment/padding we can
/// determine without interpreting DFD texel-block semantics.
fn validate_metadata_order_and_padding(
    bytes: &[u8],
    index_end: usize,
    dfd: Range,
    kvd: Range,
    sgd: Range,
) -> Result<(), Ktx2Error> {
    if dfd.start != index_end {
        return Err(Ktx2Error::InvalidSectionOrder);
    }
    if kvd.end > kvd.start && kvd.start != dfd.end {
        return Err(Ktx2Error::InvalidSectionOrder);
    }
    let previous_end = if kvd.end > kvd.start {
        kvd.end
    } else {
        dfd.end
    };
    if sgd.end > sgd.start {
        let expected_start = align_up(previous_end, 8).ok_or(Ktx2Error::SectionOutOfBounds)?;
        if sgd.start != expected_start {
            return Err(Ktx2Error::InvalidSectionOrder);
        }
        validate_zero_padding(bytes, previous_end, sgd.start)?;
    }
    Ok(())
}

fn align_up(value: usize, alignment: usize) -> Option<usize> {
    value
        .checked_add(alignment - 1)
        .map(|aligned| aligned & !(alignment - 1))
}

fn validate_zero_padding(bytes: &[u8], start: usize, end: usize) -> Result<(), Ktx2Error> {
    if start > end || end > bytes.len() {
        return Err(Ktx2Error::InvalidPadding);
    }
    if bytes[start..end].iter().any(|byte| *byte != 0) {
        return Err(Ktx2Error::InvalidPadding);
    }
    Ok(())
}

fn validate_level_padding(
    bytes: &[u8],
    doc: &Ktx2Document<'_>,
    dfd: Range,
    kvd: Range,
    sgd: Range,
) -> Result<(), Ktx2Error> {
    let metadata_end = dfd.end.max(kvd.end).max(sgd.end);
    for index in 0..doc.level_count {
        let range = doc.level_range(index)?;
        let mut previous_end = metadata_end;
        for other_index in 0..doc.level_count {
            if index == other_index {
                continue;
            }
            let other = doc.level_range(other_index)?;
            if other.end <= range.start && other.end > previous_end {
                previous_end = other.end;
            }
        }
        if doc.supercompression_scheme != 0 && range.start != previous_end {
            return Err(Ktx2Error::InvalidPadding);
        }
        validate_zero_padding(bytes, previous_end, range.start)?;
    }
    Ok(())
}

fn read_section(
    bytes: &[u8],
    offset_field: usize,
    length_field: usize,
    alignment: u64,
    required: bool,
) -> Result<Range, Ktx2Error> {
    let offset = if length_field == 72 {
        u64_at(bytes, offset_field)
    } else {
        u32_at(bytes, offset_field) as u64
    };
    let length = if length_field == 72 {
        u64_at(bytes, length_field)
    } else {
        u32_at(bytes, length_field) as u64
    };
    if (offset == 0) != (length == 0) || (required && length == 0) {
        return Err(Ktx2Error::InvalidSectionPair);
    }
    if length == 0 {
        return Ok(Range::EMPTY);
    }
    if offset % alignment != 0 {
        return Err(Ktx2Error::InvalidSectionAlignment);
    }
    let end = offset
        .checked_add(length)
        .ok_or(Ktx2Error::SectionOutOfBounds)?;
    let start = usize::try_from(offset).map_err(|_| Ktx2Error::PlatformAddressOverflow)?;
    let end = usize::try_from(end).map_err(|_| Ktx2Error::PlatformAddressOverflow)?;
    if start > end || end > bytes.len() {
        return Err(Ktx2Error::SectionOutOfBounds);
    }
    Ok(Range { start, end })
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}

#[cfg(test)]
#[path = "texture_ktx2_tests.rs"]
mod tests;
