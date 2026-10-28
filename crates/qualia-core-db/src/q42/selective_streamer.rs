//! Installed-HMC Q42 Query-to-Byte-Range Streamer (`BS-30`, `AR-04`, `ENG-07`).
//!
//! Provides Tier-1 zero-heap selective byte-range resolution and verification
//! directly from installed `.hmc` containers into caller-allocated buffers.

use core::fmt;
use crate::container_10d::crc32c::crc32c;

pub const CONTAINER_KIND_Q42: u8 = 1 << 0; // 0x01: .q42 semantic volume / graph
pub const CONTAINER_KIND_M64: u8 = 1 << 1; // 0x02: .m64 raster / spatial chunk
pub const CONTAINER_KIND_10D: u8 = 1 << 2; // 0x04: .10d volumetric mesh / manifold
pub const CONTAINER_KIND_P64: u8 = 1 << 3; // 0x08: .p64 neural weight shard
pub const CONTAINER_KIND_ALL: u8 = 0x0F;

/// Query selector specifying filtering criteria for selective HMC streaming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SelectiveStreamQuery {
    pub spatial_cell_hash: Option<u64>,
    pub scale_band_id: Option<u8>,
    pub organ_id: Option<u64>,
    pub lod_tier: Option<u8>,
    pub container_kind_mask: u8,
}

impl SelectiveStreamQuery {
    #[inline]
    pub const fn new() -> Self {
        Self {
            spatial_cell_hash: None,
            scale_band_id: None,
            organ_id: None,
            lod_tier: None,
            container_kind_mask: CONTAINER_KIND_ALL,
        }
    }

    #[inline]
    pub const fn with_organ(mut self, organ_id: u64) -> Self {
        self.organ_id = Some(organ_id);
        self
    }

    #[inline]
    pub const fn with_lod(mut self, lod_tier: u8) -> Self {
        self.lod_tier = Some(lod_tier);
        self
    }

    #[inline]
    pub const fn with_spatial_cell(mut self, cell_hash: u64) -> Self {
        self.spatial_cell_hash = Some(cell_hash);
        self
    }

    #[inline]
    pub const fn with_scale_band(mut self, scale_band_id: u8) -> Self {
        self.scale_band_id = Some(scale_band_id);
        self
    }

    #[inline]
    pub const fn with_container_mask(mut self, mask: u8) -> Self {
        self.container_kind_mask = mask;
        self
    }

    #[inline]
    pub fn matches(&self, entry: &HmcIndexSliceEntry) -> bool {
        if self.container_kind_mask != 0 && (entry.container_kind & self.container_kind_mask) == 0 {
            return false;
        }
        if let Some(spatial) = self.spatial_cell_hash {
            if entry.spatial_cell_hash != spatial { return false; }
        }
        if let Some(band) = self.scale_band_id {
            if entry.scale_band_id != band { return false; }
        }
        if let Some(organ) = self.organ_id {
            if entry.organ_id != organ { return false; }
        }
        if let Some(lod) = self.lod_tier {
            if entry.lod_tier != lod { return false; }
        }
        true
    }
}

/// Metadata entry in the HMC index for a single sub-resource slice or chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HmcIndexSliceEntry {
    pub entry_id: u32,
    pub container_kind: u8,
    pub spatial_cell_hash: u64,
    pub scale_band_id: u8,
    pub organ_id: u64,
    pub lod_tier: u8,
    pub byte_offset: u64,
    pub byte_length: u32,
    pub expected_crc32c: u32,
}

/// A resolved byte range ready for targeted streaming and verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResolvedByteRange {
    pub entry_id: u32,
    pub container_kind: u8,
    pub byte_offset: u64,
    pub byte_length: u32,
    pub expected_crc32c: u32,
}

/// Read statistics oracle proving selective access and untouched entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StreamReadOracle {
    pub total_bytes_requested: u64,
    pub ranges_touched: usize,
    pub untouched_entries_skipped: usize,
}

/// Typed error returned during selective range streaming or integrity verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectiveStreamError {
    Cancelled,
    ReadBudgetExceeded { required: u64, budget: u64 },
    DigestMismatch { entry_id: u32, expected: u32, actual: u32 },
    OutputBufferTooSmall { required: usize, provided: usize },
    InvalidSliceBounds { offset: u64, length: u32, source_len: usize },
}

impl fmt::Display for SelectiveStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => write!(f, "streaming operation cancelled"),
            Self::ReadBudgetExceeded { required, budget } => {
                write!(f, "read budget exceeded: {required} > {budget}")
            }
            Self::DigestMismatch { entry_id, expected, actual } => {
                write!(f, "CRC-32C mismatch on entry {entry_id}: expected {expected:#010x}, got {actual:#010x}")
            }
            Self::OutputBufferTooSmall { required, provided } => {
                write!(f, "output buffer too small: required {required}, provided {provided}")
            }
            Self::InvalidSliceBounds { offset, length, source_len } => {
                write!(f, "invalid bounds [{offset}..+{length}] for source length {source_len}")
            }
        }
    }
}

impl std::error::Error for SelectiveStreamError {}

/// Tier-1 zero-heap selective HMC byte-range resolver.
pub fn resolve_selective_hmc_ranges_zero_heap(
    index: &[HmcIndexSliceEntry],
    query: &SelectiveStreamQuery,
    max_total_bytes: u64,
    cancelled: bool,
    out_ranges: &mut [ResolvedByteRange],
) -> Result<(usize, StreamReadOracle), SelectiveStreamError> {
    if cancelled {
        return Err(SelectiveStreamError::Cancelled);
    }
    let mut total_bytes_requested: u64 = 0;
    let mut ranges_touched: usize = 0;
    let mut untouched_entries_skipped: usize = 0;

    for entry in index {
        if query.matches(entry) {
            let next_total = total_bytes_requested.saturating_add(entry.byte_length as u64);
            if next_total > max_total_bytes {
                return Err(SelectiveStreamError::ReadBudgetExceeded {
                    required: next_total,
                    budget: max_total_bytes,
                });
            }
            total_bytes_requested = next_total;

            if ranges_touched >= out_ranges.len() {
                return Err(SelectiveStreamError::OutputBufferTooSmall {
                    required: ranges_touched + 1,
                    provided: out_ranges.len(),
                });
            }

            out_ranges[ranges_touched] = ResolvedByteRange {
                entry_id: entry.entry_id,
                container_kind: entry.container_kind,
                byte_offset: entry.byte_offset,
                byte_length: entry.byte_length,
                expected_crc32c: entry.expected_crc32c,
            };
            ranges_touched += 1;
        } else {
            untouched_entries_skipped += 1;
        }
    }

    Ok((
        ranges_touched,
        StreamReadOracle {
            total_bytes_requested,
            ranges_touched,
            untouched_entries_skipped,
        },
    ))
}

/// Streams and verifies a specific sub-range from an in-memory HMC slice into `out_buf`.
pub fn stream_verified_subrange_into(
    hmc_slice: &[u8],
    range: &ResolvedByteRange,
    cancelled: bool,
    out_buf: &mut [u8],
) -> Result<usize, SelectiveStreamError> {
    if cancelled {
        return Err(SelectiveStreamError::Cancelled);
    }
    let len = range.byte_length as usize;
    if out_buf.len() < len {
        return Err(SelectiveStreamError::OutputBufferTooSmall {
            required: len,
            provided: out_buf.len(),
        });
    }

    let start = usize::try_from(range.byte_offset).map_err(|_| {
        SelectiveStreamError::InvalidSliceBounds {
            offset: range.byte_offset,
            length: range.byte_length,
            source_len: hmc_slice.len(),
        }
    })?;

    let end = start.checked_add(len).ok_or(SelectiveStreamError::InvalidSliceBounds {
        offset: range.byte_offset,
        length: range.byte_length,
        source_len: hmc_slice.len(),
    })?;

    if end > hmc_slice.len() {
        return Err(SelectiveStreamError::InvalidSliceBounds {
            offset: range.byte_offset,
            length: range.byte_length,
            source_len: hmc_slice.len(),
        });
    }

    let source = &hmc_slice[start..end];
    let actual_crc = crc32c(source);

    if actual_crc != range.expected_crc32c {
        out_buf[..len].fill(0);
        return Err(SelectiveStreamError::DigestMismatch {
            entry_id: range.entry_id,
            expected: range.expected_crc32c,
            actual: actual_crc,
        });
    }

    out_buf[..len].copy_from_slice(source);
    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

    fn make_test_fixture() -> ([HmcIndexSliceEntry; 4], [u8; 1024]) {
        let mut hmc_data = [0u8; 1024];
        hmc_data[64..192].fill(0x11);
        let crc0 = crc32c(&hmc_data[64..192]);
        hmc_data[192..256].fill(0x22);
        let crc1 = crc32c(&hmc_data[192..256]);
        hmc_data[256..512].fill(0x33);
        let crc2 = crc32c(&hmc_data[256..512]);
        hmc_data[512..640].fill(0x44);
        let crc3 = crc32c(&hmc_data[512..640]);

        let index = [
            HmcIndexSliceEntry { entry_id: 1, container_kind: CONTAINER_KIND_10D, spatial_cell_hash: 10, scale_band_id: 1, organ_id: 100, lod_tier: 0, byte_offset: 64, byte_length: 128, expected_crc32c: crc0 },
            HmcIndexSliceEntry { entry_id: 2, container_kind: CONTAINER_KIND_10D, spatial_cell_hash: 10, scale_band_id: 1, organ_id: 100, lod_tier: 2, byte_offset: 192, byte_length: 64, expected_crc32c: crc1 },
            HmcIndexSliceEntry { entry_id: 3, container_kind: CONTAINER_KIND_M64, spatial_cell_hash: 20, scale_band_id: 2, organ_id: 200, lod_tier: 0, byte_offset: 256, byte_length: 256, expected_crc32c: crc2 },
            HmcIndexSliceEntry { entry_id: 4, container_kind: CONTAINER_KIND_Q42, spatial_cell_hash: 20, scale_band_id: 2, organ_id: 200, lod_tier: 1, byte_offset: 512, byte_length: 128, expected_crc32c: crc3 },
        ];
        (index, hmc_data)
    }

    #[test]
    fn selective_query_skips_unrelated_chunks_and_verifies_payload() {
        let (index, hmc_data) = make_test_fixture();
        let query = SelectiveStreamQuery::new().with_organ(100).with_lod(2);
        let mut out_ranges = [ResolvedByteRange::default(); 8];
        let (count, oracle) = resolve_selective_hmc_ranges_zero_heap(&index, &query, 1024, false, &mut out_ranges).unwrap();

        assert_eq!(count, 1);
        assert_eq!(oracle.ranges_touched, 1);
        assert_eq!(oracle.untouched_entries_skipped, 3);
        assert_eq!(oracle.total_bytes_requested, 64);
        assert_eq!(out_ranges[0].entry_id, 2);

        let mut read_buf = [0u8; 128];
        let bytes_read = stream_verified_subrange_into(&hmc_data, &out_ranges[0], false, &mut read_buf).unwrap();
        assert_eq!(bytes_read, 64);
        assert!(read_buf[..64].iter().all(|&b| b == 0x22));
    }

    #[test]
    fn budget_overflow_fails_closed_before_read() {
        let (index, _) = make_test_fixture();
        let query = SelectiveStreamQuery::new().with_organ(100);
        let mut out_ranges = [ResolvedByteRange::default(); 8];
        let err = resolve_selective_hmc_ranges_zero_heap(&index, &query, 100, false, &mut out_ranges).unwrap_err();

        match err {
            SelectiveStreamError::ReadBudgetExceeded { required, budget } => {
                assert_eq!(required, 128);
                assert_eq!(budget, 100);
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn cancellation_fails_closed() {
        let (index, hmc_data) = make_test_fixture();
        let query = SelectiveStreamQuery::new();
        let mut out_ranges = [ResolvedByteRange::default(); 8];
        let err_resolve = resolve_selective_hmc_ranges_zero_heap(&index, &query, 1024, true, &mut out_ranges).unwrap_err();
        assert_eq!(err_resolve, SelectiveStreamError::Cancelled);

        let range = ResolvedByteRange { entry_id: 1, container_kind: CONTAINER_KIND_10D, byte_offset: 64, byte_length: 128, expected_crc32c: 0x12345678 };
        let mut buf = [0u8; 128];
        let err_stream = stream_verified_subrange_into(&hmc_data, &range, true, &mut buf).unwrap_err();
        assert_eq!(err_stream, SelectiveStreamError::Cancelled);
    }

    #[test]
    fn tampered_byte_triggers_digest_mismatch_and_zeroes_buffer() {
        let (index, mut hmc_data) = make_test_fixture();
        let query = SelectiveStreamQuery::new().with_organ(100).with_lod(0);
        let mut out_ranges = [ResolvedByteRange::default(); 4];
        let (count, _) = resolve_selective_hmc_ranges_zero_heap(&index, &query, 1024, false, &mut out_ranges).unwrap();
        assert_eq!(count, 1);

        hmc_data[100] ^= 0xA5;
        let mut out_buf = [0xFFu8; 128];
        let err = stream_verified_subrange_into(&hmc_data, &out_ranges[0], false, &mut out_buf).unwrap_err();

        match err {
            SelectiveStreamError::DigestMismatch { entry_id, expected, actual } => {
                assert_eq!(entry_id, 1);
                assert_ne!(expected, actual);
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
        assert!(out_buf.iter().all(|&b| b == 0));
    }

    #[test]
    #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
    fn zero_alloc_streamer_resolution_and_read() {
        let (index, hmc_data) = make_test_fixture();
        let query = SelectiveStreamQuery::new().with_organ(100).with_lod(0);
        let mut out_ranges = [ResolvedByteRange::default(); 4];
        let mut out_buf = [0u8; 128];
        let _ = crc32c(b"warmup");

        assert_zero_alloc("resolve_selective_hmc_ranges_zero_heap", || {
            let res = resolve_selective_hmc_ranges_zero_heap(&index, &query, 1024, false, &mut out_ranges);
            assert!(res.is_ok());
            let read_res = stream_verified_subrange_into(&hmc_data, &out_ranges[0], false, &mut out_buf);
            assert!(read_res.is_ok());
        });
    }
}
