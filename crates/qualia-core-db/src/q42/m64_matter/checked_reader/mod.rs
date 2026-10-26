//! Bounded structural reader for the existing little-endian M64/E64 layouts.
//!
//! Header and directory PODs are copied to aligned stack values; payload bytes
//! remain borrowed. No authentication, Bao proof, domain or scientific validity
//! is established. Point/holo records have no embedded CRC in these layouts.
//! Caller limits and interval scratch bound work without allocation or recursion.

mod intervals;
mod validation;

use super::{M64ChunkDirectoryEntry, M64Error, M64Header};
use super::{M64GeoRasterError, M64GeoRasterMetadata, M64RasterSample};

/// Explicit admission budgets; there is no implicit unlimited/default policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct M64ReadLimits {
    pub max_file_bytes: usize,
    pub max_chunks: usize,
    pub max_records: u64,
}

/// Caller-owned overlap scratch. One element per directory entry is required.
/// Contents are unspecified after parsing, including on error.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct M64PayloadInterval {
    start: usize,
    end: usize,
}

/// All header/directory CRCs and structure passed. Only strata/phase records
/// carry record CRCs; point/holo records are counted separately, never certified.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct M64Verification {
    pub chunks: usize,
    pub total_records: u64,
    pub record_crc_checked: u64,
    pub records_without_checksum: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum M64ReadError {
    GeoRaster(M64GeoRasterError),
    UnsupportedEndian,
    FileByteLimit,
    TruncatedHeader,
    Header(M64Error),
    UnsupportedVersion(u32),
    UnsupportedContainerFlags(u32),
    FileSizeMismatch,
    ChunkLimit,
    RecordLimit,
    ScratchTooSmall {
        required: usize,
        provided: usize,
    },
    ArithmeticOverflow,
    DirectoryBounds,
    DirectoryAlignment,
    Directory {
        chunk: usize,
        error: M64Error,
    },
    UnsupportedMode {
        chunk: usize,
        mode: u8,
    },
    UnsupportedCodec {
        chunk: usize,
        codec: u8,
    },
    PayloadBounds {
        chunk: usize,
    },
    PayloadAlignment {
        chunk: usize,
    },
    RecordLength {
        chunk: usize,
    },
    MetadataOverlap {
        chunk: usize,
    },
    PayloadOverlap,
    Record {
        chunk: usize,
        record: usize,
        error: M64Error,
    },
}

impl std::fmt::Display for M64ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for M64ReadError {}

/// A fully structurally checked immutable file view. Qualification is native
/// little-endian; big-endian POD interpretation is rejected explicitly.
#[derive(Debug)]
pub struct M64Reader<'a> {
    bytes: &'a [u8],
    header: M64Header,
    directory_start: usize,
    verification: M64Verification,
    georaster: Option<M64GeoRasterMetadata>,
}

impl<'a> M64Reader<'a> {
    /// Admit the entire file. All metadata and budgets are checked before record
    /// CRC work. Scratch sorting is iterative O(chunks log chunks), not quadratic.
    /// v1/v2 and both m64/e64 magics retain their existing wire interpretation.
    /// Only raw codec 0 is supported; flags never imply a verified Bao proof.
    pub fn parse(
        bytes: &'a [u8],
        limits: M64ReadLimits,
        scratch: &mut [M64PayloadInterval],
    ) -> Result<Self, M64ReadError> {
        validation::parse(bytes, limits, scratch)
    }

    pub fn header(&self) -> &M64Header {
        &self.header
    }

    /// Exact v3 metadata; `None` for v1/v2 without inferring missing semantics.
    pub fn georaster(&self) -> Option<&M64GeoRasterMetadata> {
        self.georaster.as_ref()
    }

    /// Source coordinates only: `None` outside source W/H, including padding,
    /// or for legacy containers. Declared nodata is distinct from elevation.
    pub fn raster_sample(&self, x: usize, y: usize) -> Option<M64RasterSample> {
        let meta = self.georaster.as_ref()?;
        let shape = meta.shape().ok()?;
        if x >= shape.width || y >= shape.height {
            return None;
        }
        let chunk = self.chunk((y / 64) * shape.chunks_x + x / 64)?;
        let raw = chunk.record_bytes((y % 64) * 64 + x % 64)?;
        let value = i32::from_le_bytes(raw[..4].try_into().ok()?);
        Some(if meta.nodata_sentinel_mm == Some(value) {
            M64RasterSample::Nodata(value)
        } else {
            M64RasterSample::ElevationMm(value)
        })
    }

    pub fn verification(&self) -> M64Verification {
        self.verification
    }

    /// Read a checked directory row and borrow its raw payload, irrespective of
    /// input pointer alignment. Serialized offsets remain 64-byte aligned.
    pub fn chunk(&self, index: usize) -> Option<M64Chunk<'a>> {
        if index >= self.verification.chunks {
            return None;
        }
        let offset = self.directory_start + index * 64;
        let entry: M64ChunkDirectoryEntry =
            bytemuck::pod_read_unaligned(&self.bytes[offset..offset + 64]);
        let start = entry.byte_offset as usize;
        let end = start + entry.byte_length as usize;
        Some(M64Chunk {
            entry,
            payload: &self.bytes[start..end],
        })
    }
}

#[derive(Debug)]
pub struct M64Chunk<'a> {
    entry: M64ChunkDirectoryEntry,
    payload: &'a [u8],
}

impl<'a> M64Chunk<'a> {
    pub fn directory_entry(&self) -> &M64ChunkDirectoryEntry {
        &self.entry
    }

    /// Borrowed bytes, not an aligned typed slice. This is the zero-copy scope.
    pub fn payload(&self) -> &'a [u8] {
        self.payload
    }

    pub fn record_bytes(&self, index: usize) -> Option<&'a [u8]> {
        let start = index.checked_mul(64)?;
        self.payload.get(start..start.checked_add(64)?)
    }
}
