//! Byte-backed `.q42` volume and raw `NQuin` slice reader for WASM and native hosts.
//!
//! Operates directly over `&[u8]` (such as an `.hmc` / `.qpack` entry slice or
//! OPFS buffer) without requiring `std::fs::File` or `memmap2`. Hot-path block
//! queries, object BIDX searches, and lexicon lookups write into caller-supplied
//! slices with zero heap allocations.

use std::io;

use super::super::{
    MAX_COMPRESSED_SUPERBLOCK_SIZE, Q42_MAGIC, QUIN_SIZE, SUPERBLOCK_SIZE,
};
use super::opfs_source::OpfsSliceRangeSource;
use super::range::{Q42ByteRange, Q42RangeSource};
use super::range_volume::{
    Q42ObjectMatchPage, Q42ObjectSearchCursor, Q42RangeQueryCursor, Q42RangeQueryPage,
    Q42RangeQueryPattern, Q42RangeQueryPlan, Q42RangeVolume,
};
use crate::q42_lex::{
    INDEX_ENTRY_SIZE, LEX_HEADER_SIZE, LEX_MAGIC, LEX_TAG_STRING, LEX_VERSION_PAGED,
    LEX_VERSION_V4,
};
use crate::NQuin;

fn invalid(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

enum SliceBacking<'a> {
    UnifiedVolume(Q42RangeVolume<OpfsSliceRangeSource<&'a [u8]>>),
    RawQuinSlice(&'a [u8]),
}

/// Transport-neutral reader over a borrowed `.q42` volume or 48-byte `NQuin` slice.
pub struct WasmSliceQ42Reader<'a> {
    bytes: &'a [u8],
    backing: SliceBacking<'a>,
}

impl<'a> WasmSliceQ42Reader<'a> {
    /// Open and validate a `.q42` volume slice or a raw 48-byte `NQuin` array slice.
    pub fn open(bytes: &'a [u8]) -> io::Result<Self> {
        if bytes.is_empty() {
            return Err(invalid("empty Q42 byte slice"));
        }
        if bytes.len() >= 4 && bytes[0..4] == Q42_MAGIC {
            let source = OpfsSliceRangeSource::new(bytes);
            let volume = Q42RangeVolume::open(source)?;
            return Ok(Self {
                bytes,
                backing: SliceBacking::UnifiedVolume(volume),
            });
        }
        if bytes.len() % QUIN_SIZE == 0 {
            for chunk in bytes.chunks_exact(QUIN_SIZE) {
                let quin = bytemuck::pod_read_unaligned::<NQuin>(chunk);
                if !quin.verify_ecc_parity() {
                    return Err(invalid("raw NQuin slice failed parity integrity check"));
                }
            }
            return Ok(Self {
                bytes,
                backing: SliceBacking::RawQuinSlice(bytes),
            });
        }
        Err(invalid(
            "byte slice is neither a Q42VOL unified volume nor a 48-byte-aligned NQuin array",
        ))
    }

    /// Returns true if backed by a full `Q42VOL` headered volume rather than raw 48B quins.
    #[inline]
    pub fn is_unified_volume(&self) -> bool {
        matches!(self.backing, SliceBacking::UnifiedVolume(_))
    }

    /// Version number of the volume (`3` or `4` for unified `.q42`, `0` for raw `NQuin` slice).
    #[inline]
    pub fn version(&self) -> u32 {
        match &self.backing {
            SliceBacking::UnifiedVolume(vol) => u32::from(vol.header().version),
            SliceBacking::RawQuinSlice(_) => 0,
        }
    }

    /// Number of SuperBlocks (`0` for raw `NQuin` slices).
    #[inline]
    pub fn block_count(&self) -> u64 {
        match &self.backing {
            SliceBacking::UnifiedVolume(vol) => vol.block_count(),
            SliceBacking::RawQuinSlice(_) => 0,
        }
    }

    /// Underlying byte slice length.
    #[inline]
    pub fn byte_len(&self) -> usize {
        self.bytes.len()
    }

    /// Zero-heap paged pattern query into caller-supplied buffers.
    ///
    /// For unified `.q42` volumes, `compressed` must be at least
    /// `MAX_COMPRESSED_SUPERBLOCK_SIZE` and `decoded` must be at least `SUPERBLOCK_SIZE`.
    pub fn execute_query_page_into(
        &self,
        plan: Q42RangeQueryPlan,
        cursor: Q42RangeQueryCursor,
        compressed: &mut [u8],
        decoded: &mut [u8],
        out: &mut [NQuin],
    ) -> io::Result<Q42RangeQueryPage> {
        match &self.backing {
            SliceBacking::UnifiedVolume(vol) => {
                vol.execute_query_page_into(plan, cursor, compressed, decoded, out)
            }
            SliceBacking::RawQuinSlice(raw) => {
                if out.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Q42 query output buffer is empty",
                    ));
                }
                let total_quins = raw.len() / QUIN_SIZE;
                if cursor.block_index > 0 || cursor.quin_offset >= total_quins {
                    return Ok(Q42RangeQueryPage {
                        returned: 0,
                        next_cursor: None,
                    });
                }
                let mut idx = cursor.quin_offset;
                let mut returned = 0usize;
                let pat = plan.pattern;
                while idx < total_quins {
                    let start = idx * QUIN_SIZE;
                    let quin = bytemuck::pod_read_unaligned::<NQuin>(&raw[start..start + QUIN_SIZE]);
                    idx += 1;
                    if pat.subject.is_some_and(|s| s != quin.subject)
                        || pat.predicate.is_some_and(|p| p != quin.predicate)
                        || pat.object.is_some_and(|o| o != quin.object)
                        || pat.context.is_some_and(|c| c != quin.context)
                    {
                        continue;
                    }
                    out[returned] = quin;
                    returned += 1;
                    if returned == out.len() {
                        let next_cursor = (idx < total_quins).then_some(Q42RangeQueryCursor {
                            block_index: 0,
                            quin_offset: idx,
                        });
                        return Ok(Q42RangeQueryPage {
                            returned,
                            next_cursor,
                        });
                    }
                }
                Ok(Q42RangeQueryPage {
                    returned,
                    next_cursor: None,
                })
            }
        }
    }

    /// Zero-heap exact object lookup using BIDX on unified volumes or linear scan on raw slices.
    pub fn find_object_into(
        &self,
        object_hash: u64,
        cursor: Q42ObjectSearchCursor,
        compressed: &mut [u8],
        decoded: &mut [u8],
        out: &mut [NQuin],
    ) -> io::Result<Option<Q42ObjectMatchPage>> {
        match &self.backing {
            SliceBacking::UnifiedVolume(vol) => {
                vol.find_object_into(object_hash, cursor, compressed, decoded, out)
            }
            SliceBacking::RawQuinSlice(_) => {
                let plan = Q42RangeQueryPlan::for_pattern(Q42RangeQueryPattern {
                    subject: None,
                    predicate: None,
                    object: Some(object_hash),
                    context: None,
                });
                let page = self.execute_query_page_into(
                    plan,
                    Q42RangeQueryCursor {
                        block_index: cursor.block_offset,
                        quin_offset: cursor.quin_offset,
                    },
                    compressed,
                    decoded,
                    out,
                )?;
                if page.returned == 0 && cursor.block_offset == 0 && cursor.quin_offset == 0 {
                    return Ok(None);
                }
                Ok(Some(Q42ObjectMatchPage {
                    block_range: super::index::BidxBlockRange { start: 0, end: 1 },
                    returned: page.returned,
                    next_cursor: page.next_cursor.map(|c| Q42ObjectSearchCursor {
                        block_offset: c.block_index,
                        quin_offset: c.quin_offset,
                    }),
                }))
            }
        }
    }

    /// Zero-heap lexicon hash lookup supporting both flat v1 `Q42LEX` and paged v2/v4 `Q42LEX`.
    pub fn lookup_lexicon_hash_into(
        &self,
        hash: u64,
        page_scratch: &mut [u8],
        out: &mut [u8],
    ) -> io::Result<Option<usize>> {
        let SliceBacking::UnifiedVolume(vol) = &self.backing else {
            return Ok(None);
        };
        let lex_offset = vol.header().lex_offset;
        let lex_length = vol.header().lex_length;
        if lex_length < LEX_HEADER_SIZE as u64 {
            return Ok(None);
        }
        let mut header = [0u8; LEX_HEADER_SIZE];
        vol.source().read_range_into(
            Q42ByteRange {
                offset: lex_offset,
                length: LEX_HEADER_SIZE,
            },
            &mut header,
        )?;
        if header[0..8] != LEX_MAGIC {
            return Err(invalid("Q42 lexicon section has invalid magic"));
        }
        let version = u64::from_le_bytes(header[24..32].try_into().unwrap());
        if version == LEX_VERSION_PAGED || version == LEX_VERSION_V4 {
            return vol.lookup_lexicon_hash_into(hash, page_scratch, out);
        }
        if version != 1 {
            return Err(invalid("unsupported Q42LEX version"));
        }
        let entry_count = u64::from_le_bytes(header[8..16].try_into().unwrap()) as usize;
        let strings_offset = u64::from_le_bytes(header[16..24].try_into().unwrap()) as usize;
        let lex_start = lex_offset as usize;
        let lex_end = lex_start
            .checked_add(lex_length as usize)
            .ok_or_else(|| invalid("Q42 lexicon slice overflow"))?;
        if lex_end > self.bytes.len() {
            return Err(invalid("Q42 lexicon extends past volume slice"));
        }
        let lex_bytes = &self.bytes[lex_start..lex_end];
        let index_end = LEX_HEADER_SIZE
            .checked_add(entry_count.saturating_mul(INDEX_ENTRY_SIZE))
            .ok_or_else(|| invalid("Q42 lexicon index overflow"))?;
        if strings_offset < index_end || strings_offset > lex_bytes.len() {
            return Err(invalid("Q42 v1 lexicon strings offset out of bounds"));
        }
        let mut lo = 0usize;
        let mut hi = entry_count;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let pos = LEX_HEADER_SIZE + mid * INDEX_ENTRY_SIZE;
            let entry_hash = u64::from_le_bytes(lex_bytes[pos..pos + 8].try_into().unwrap());
            if entry_hash < hash {
                lo = mid + 1;
            } else if entry_hash > hash {
                hi = mid;
            } else {
                let rel = u64::from_le_bytes(lex_bytes[pos + 8..pos + 16].try_into().unwrap())
                    as usize;
                let start = strings_offset
                    .checked_add(rel)
                    .ok_or_else(|| invalid("Q42 lexicon string offset overflow"))?;
                if start + 3 > lex_bytes.len() || lex_bytes[start] != LEX_TAG_STRING {
                    return Err(invalid("Q42 lexicon v1 entry malformed"));
                }
                let str_len =
                    u16::from_le_bytes(lex_bytes[start + 1..start + 3].try_into().unwrap())
                        as usize;
                let end = start + 3 + str_len;
                if end > lex_bytes.len() {
                    return Err(invalid("Q42 lexicon v1 string truncated"));
                }
                if str_len > out.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "Q42 lexicon output buffer too small",
                    ));
                }
                std::str::from_utf8(&lex_bytes[start + 3..end])
                    .map_err(|_| invalid("Q42 lexicon v1 entry is not UTF-8"))?;
                out[..str_len].copy_from_slice(&lex_bytes[start + 3..end]);
                return Ok(Some(str_len));
            }
        }
        Ok(None)
    }

    /// Cold-path helper to collect all `NQuin` records from the slice.
    pub fn collect_all_quins(&self) -> io::Result<Vec<NQuin>> {
        match &self.backing {
            SliceBacking::RawQuinSlice(raw) => {
                let mut out = Vec::with_capacity(raw.len() / QUIN_SIZE);
                for chunk in raw.chunks_exact(QUIN_SIZE) {
                    out.push(bytemuck::pod_read_unaligned::<NQuin>(chunk));
                }
                Ok(out)
            }
            SliceBacking::UnifiedVolume(vol) => {
                let mut compressed = vec![0u8; MAX_COMPRESSED_SUPERBLOCK_SIZE];
                let mut decoded = vec![0u8; SUPERBLOCK_SIZE];
                let mut page_buf = [NQuin::default(); 256];
                let mut all = Vec::new();
                let plan = Q42RangeQueryPlan::for_pattern(Q42RangeQueryPattern::default());
                let mut cursor = Q42RangeQueryCursor::default();
                loop {
                    let page = vol.execute_query_page_into(
                        plan,
                        cursor,
                        &mut compressed,
                        &mut decoded,
                        &mut page_buf,
                    )?;
                    all.extend_from_slice(&page_buf[..page.returned]);
                    match page.next_cursor {
                        Some(next) => cursor = next,
                        None => break,
                    }
                }
                Ok(all)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

    #[test]
    fn raw_quin_slice_reader_queries_zero_heap() {
        let mut q1 = NQuin {
            subject: 11,
            predicate: 22,
            object: 33,
            context: 44,
            metadata: 0,
            parity: 0,
        };
        q1.parity =
            NQuin::calculate_parity(q1.subject, q1.predicate, q1.object, q1.context, q1.metadata);
        let mut q2 = NQuin {
            subject: 11,
            predicate: 22,
            object: 99,
            context: 44,
            metadata: 0,
            parity: 0,
        };
        q2.parity =
            NQuin::calculate_parity(q2.subject, q2.predicate, q2.object, q2.context, q2.metadata);
        let quins = [q1, q2];
        let raw_bytes: &[u8] = bytemuck::cast_slice(&quins);

        let reader = WasmSliceQ42Reader::open(raw_bytes).expect("open raw slice");
        assert!(!reader.is_unified_volume());

        let mut comp = [0u8; 64];
        let mut dec = [0u8; 64];
        let mut out = [NQuin::default(); 4];
        let mut matched = 0usize;

        assert_zero_alloc("wasm_slice_raw_find_object", || {
            let page = reader
                .find_object_into(
                    99,
                    Q42ObjectSearchCursor::default(),
                    &mut comp,
                    &mut dec,
                    &mut out,
                )
                .unwrap()
                .unwrap();
            matched = page.returned;
        });
        assert_eq!(matched, 1);
        assert_eq!(out[0].object, 99);
    }
}
