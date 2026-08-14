//! Range-backed Q42 segment reader for local, HTTP, and IPFS sources.

use std::io;

use super::super::{
    header_from_bytes, BlockDirectoryEntry, Q42VolumeHeader, BIDX_MAGIC, FLAG_BLOCKS_LZ4,
    HEADER_SIZE, MAX_COMPRESSED_SUPERBLOCK_SIZE, Q42_VERSION_V3, QUINS_PER_BLOCK, QUIN_SIZE,
    SUPERBLOCK_HEADER, SUPERBLOCK_SIZE,
};
use super::index::{BidxBlockRange, BidxMatchPage};
use super::range::{Q42ByteRange, Q42RangeSource};
use crate::NQuin;

const BIDX_HEADER_BYTES: usize = 16;
const BIDX_ENTRY_BYTES: usize = 16;

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// A Q42 reader that fetches exactly the bytes needed from a random-access
/// source. All variable-size buffers remain caller-owned.
pub struct Q42RangeVolume<S: Q42RangeSource> {
    source: S,
    header: Q42VolumeHeader,
    source_length: u64,
}

/// Resume state for a caller-buffered object search. It is tied to the object
/// hash passed to [`Q42RangeVolume::find_object_into`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Q42ObjectSearchCursor {
    /// Offset into the matching BIDX block interval.
    pub block_offset: usize,
    /// Quin offset in the current decoded block.
    pub quin_offset: usize,
}

/// One page of exact object matches written by [`Q42RangeVolume::find_object_into`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Q42ObjectMatchPage {
    pub block_range: BidxBlockRange,
    pub returned: usize,
    pub next_cursor: Option<Q42ObjectSearchCursor>,
}

/// A simple physical pattern for range-backed Q42 scans. `None` is an
/// unbound SPARQL position.  The planner selects the BIDX object index when
/// `object` is bound and otherwise performs a bounded sequential block scan.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Q42RangeQueryPattern {
    pub subject: Option<u64>,
    pub predicate: Option<u64>,
    pub object: Option<u64>,
    pub context: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Q42RangeQueryStrategy {
    ObjectBidx,
    Sequential,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Q42RangeQueryPlan {
    pub pattern: Q42RangeQueryPattern,
    pub strategy: Q42RangeQueryStrategy,
}

impl Q42RangeQueryPlan {
    pub fn for_pattern(pattern: Q42RangeQueryPattern) -> Self {
        Self {
            strategy: if pattern.object.is_some() {
                Q42RangeQueryStrategy::ObjectBidx
            } else {
                Q42RangeQueryStrategy::Sequential
            },
            pattern,
        }
    }
}

/// Resume state for a bounded range-query page.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Q42RangeQueryCursor {
    pub block_index: usize,
    pub quin_offset: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Q42RangeQueryPage {
    pub returned: usize,
    pub next_cursor: Option<Q42RangeQueryCursor>,
}

impl<S: Q42RangeSource> Q42RangeVolume<S> {
    pub fn open(source: S) -> io::Result<Self> {
        let source_length = source.length()?;
        if source_length < HEADER_SIZE as u64 {
            return Err(invalid("Q42 range source is shorter than its header"));
        }
        let mut bytes = [0u8; HEADER_SIZE];
        source.read_range_into(
            Q42ByteRange {
                offset: 0,
                length: HEADER_SIZE,
            },
            &mut bytes,
        )?;
        let header = header_from_bytes(&bytes)?;
        let version = header.version;
        let flags = header.flags;
        let block_size = header.block_size;
        let quins_per_block = header.quins_per_block;
        if version != Q42_VERSION_V3
            || flags & FLAG_BLOCKS_LZ4 == 0
            || block_size != SUPERBLOCK_SIZE as u32
            || quins_per_block != QUINS_PER_BLOCK as u32
        {
            return Err(invalid("unsupported Q42 range-volume header"));
        }
        for (name, offset, length) in [
            ("lexicon", header.lex_offset, header.lex_length),
            ("BIDX", header.bidx_offset, header.bidx_length),
            (
                "block directory",
                header.block_dir_offset,
                header.block_dir_length,
            ),
            ("block data", header.data_offset, header.data_length),
        ] {
            if length != 0
                && (offset < HEADER_SIZE as u64
                    || offset
                        .checked_add(length)
                        .is_none_or(|end| end > source_length))
            {
                return Err(invalid(format!(
                    "Q42 {name} section lies outside the range source"
                )));
            }
        }
        let expected_directory = header
            .block_count
            .checked_mul(BlockDirectoryEntry::SIZE as u64)
            .ok_or_else(|| invalid("Q42 directory length overflow"))?;
        if header.block_dir_length != expected_directory {
            return Err(invalid("Q42 directory does not match its block count"));
        }
        let volume = Self {
            source,
            header,
            source_length,
        };
        volume.bidx_block_count()?;
        Ok(volume)
    }

    pub fn header(&self) -> &Q42VolumeHeader {
        &self.header
    }
    pub fn source_length(&self) -> u64 {
        self.source_length
    }
    pub fn block_count(&self) -> u64 {
        self.header.block_count
    }

    /// Execute one caller-buffered page of a physical Q42 scan.  A bound
    /// object uses BIDX to avoid unrelated SuperBlocks; all other patterns
    /// stream one block at a time.  This is the reusable low-level path that
    /// a SPARQL planner can drive without materialising a graph snapshot.
    pub fn execute_query_page_into(
        &self,
        plan: Q42RangeQueryPlan,
        cursor: Q42RangeQueryCursor,
        compressed: &mut [u8],
        decoded: &mut [u8],
        out: &mut [NQuin],
    ) -> io::Result<Q42RangeQueryPage> {
        if out.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 query output buffer is empty",
            ));
        }
        let (start, end) = match plan.strategy {
            Q42RangeQueryStrategy::ObjectBidx => match self.bidx_block_range_for_hash(
                plan.pattern
                    .object
                    .expect("object strategy requires object"),
            )? {
                Some(range) => (range.start, range.end),
                None => {
                    return Ok(Q42RangeQueryPage {
                        returned: 0,
                        next_cursor: None,
                    })
                }
            },
            Q42RangeQueryStrategy::Sequential => (0, self.header.block_count as usize),
        };
        let mut block_index = cursor.block_index.max(start);
        let mut quin_offset = if block_index == cursor.block_index {
            cursor.quin_offset
        } else {
            0
        };
        let mut returned = 0usize;
        while block_index < end {
            self.read_superblock_into(block_index, compressed, decoded)?;
            let count = u64::from_le_bytes(decoded[16..24].try_into().unwrap()) as usize;
            if count > QUINS_PER_BLOCK {
                return Err(invalid("Q42 decoded SuperBlock has invalid Quin count"));
            }
            while quin_offset < count {
                let offset = SUPERBLOCK_HEADER + quin_offset * QUIN_SIZE;
                let quin = NQuin {
                    subject: u64::from_le_bytes(decoded[offset..offset + 8].try_into().unwrap()),
                    predicate: u64::from_le_bytes(
                        decoded[offset + 8..offset + 16].try_into().unwrap(),
                    ),
                    object: u64::from_le_bytes(
                        decoded[offset + 16..offset + 24].try_into().unwrap(),
                    ),
                    context: u64::from_le_bytes(
                        decoded[offset + 24..offset + 32].try_into().unwrap(),
                    ),
                    metadata: u64::from_le_bytes(
                        decoded[offset + 32..offset + 40].try_into().unwrap(),
                    ),
                    parity: u64::from_le_bytes(
                        decoded[offset + 40..offset + 48].try_into().unwrap(),
                    ),
                };
                quin_offset += 1;
                let pattern = plan.pattern;
                if pattern.subject.is_some_and(|value| value != quin.subject)
                    || pattern
                        .predicate
                        .is_some_and(|value| value != quin.predicate)
                    || pattern.object.is_some_and(|value| value != quin.object)
                    || pattern.context.is_some_and(|value| value != quin.context)
                {
                    continue;
                }
                out[returned] = quin;
                returned += 1;
                if returned == out.len() {
                    let next = if quin_offset < count {
                        Some(Q42RangeQueryCursor {
                            block_index,
                            quin_offset,
                        })
                    } else if block_index + 1 < end {
                        Some(Q42RangeQueryCursor {
                            block_index: block_index + 1,
                            quin_offset: 0,
                        })
                    } else {
                        None
                    };
                    return Ok(Q42RangeQueryPage {
                        returned,
                        next_cursor: next,
                    });
                }
            }
            block_index += 1;
            quin_offset = 0;
        }
        Ok(Q42RangeQueryPage {
            returned,
            next_cursor: None,
        })
    }
    pub fn source(&self) -> &S {
        &self.source
    }

    pub fn read_lexicon_into(&self, out: &mut [u8]) -> io::Result<()> {
        self.read_section(self.header.lex_offset, self.header.lex_length, out)
    }

    /// Resolve a string from a paged Q42LEX dictionary using exact range reads.
    /// `page_scratch` and `out` are caller-owned; neither a full lexicon nor a
    /// decoded page is retained by the reader.  Returns the UTF-8 byte length
    /// written into `out`, or `None` when the hash is absent.
    pub fn lookup_lexicon_hash_into(
        &self,
        hash: u64,
        page_scratch: &mut [u8],
        out: &mut [u8],
    ) -> io::Result<Option<usize>> {
        use crate::q42_lex::{
            LEX_HEADER_SIZE, LEX_MAGIC, LEX_VERSION_PAGED, PAGED_DIRECTORY_ENTRY_SIZE,
            PAGED_DIRECTORY_HEADER_SIZE, PAGED_PAGE_HEADER_SIZE,
        };
        if self.header.lex_length < (LEX_HEADER_SIZE + PAGED_DIRECTORY_HEADER_SIZE) as u64 {
            return Err(invalid("Q42 lexicon is too short for paged lookup"));
        }
        let mut header = [0u8; LEX_HEADER_SIZE];
        self.source.read_range_into(
            Q42ByteRange {
                offset: self.header.lex_offset,
                length: LEX_HEADER_SIZE,
            },
            &mut header,
        )?;
        if header[0..8] != LEX_MAGIC
            || u64::from_le_bytes(header[24..32].try_into().unwrap()) != LEX_VERSION_PAGED
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Q42 range lexicon is not paged Q42LEX v2",
            ));
        }
        let directory_offset = u64::from_le_bytes(header[16..24].try_into().unwrap());
        if directory_offset != LEX_HEADER_SIZE as u64 {
            return Err(invalid("Q42 paged lexicon has an invalid directory offset"));
        }
        let mut page_count_bytes = [0u8; PAGED_DIRECTORY_HEADER_SIZE];
        self.source.read_range_into(
            Q42ByteRange {
                offset: self.header.lex_offset + directory_offset,
                length: PAGED_DIRECTORY_HEADER_SIZE,
            },
            &mut page_count_bytes,
        )?;
        let page_count = usize::try_from(u64::from_le_bytes(page_count_bytes))
            .map_err(|_| invalid("Q42 paged lexicon page count exceeds platform"))?;
        let mut lo = 0usize;
        let mut hi = page_count;
        let mut entry = [0u8; PAGED_DIRECTORY_ENTRY_SIZE];
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let offset = self.header.lex_offset
                + directory_offset
                + PAGED_DIRECTORY_HEADER_SIZE as u64
                + (mid * PAGED_DIRECTORY_ENTRY_SIZE) as u64;
            self.source.read_range_into(
                Q42ByteRange {
                    offset,
                    length: PAGED_DIRECTORY_ENTRY_SIZE,
                },
                &mut entry,
            )?;
            if u64::from_le_bytes(entry[0..8].try_into().unwrap()) <= hash {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let Some(page_index) = lo.checked_sub(1) else {
            return Ok(None);
        };
        let offset = self.header.lex_offset
            + directory_offset
            + PAGED_DIRECTORY_HEADER_SIZE as u64
            + (page_index * PAGED_DIRECTORY_ENTRY_SIZE) as u64;
        self.source.read_range_into(
            Q42ByteRange {
                offset,
                length: PAGED_DIRECTORY_ENTRY_SIZE,
            },
            &mut entry,
        )?;
        let page_offset = u64::from_le_bytes(entry[8..16].try_into().unwrap());
        let page_length = usize::try_from(u64::from_le_bytes(entry[16..24].try_into().unwrap()))
            .map_err(|_| invalid("Q42 lexicon page exceeds platform"))?;
        let count = u32::from_le_bytes(entry[24..28].try_into().unwrap()) as usize;
        if page_length > page_scratch.len()
            || page_length < PAGED_PAGE_HEADER_SIZE
            || page_offset
                .checked_add(page_length as u64)
                .is_none_or(|end| end > self.header.lex_length)
        {
            return Err(invalid(
                "Q42 lexicon page is out of bounds or exceeds scratch",
            ));
        }
        let page = &mut page_scratch[..page_length];
        self.source.read_range_into(
            Q42ByteRange {
                offset: self.header.lex_offset + page_offset,
                length: page_length,
            },
            page,
        )?;
        let declared_count = u32::from_le_bytes(page[0..4].try_into().unwrap()) as usize;
        let blob_offset = usize::try_from(u64::from_le_bytes(page[8..16].try_into().unwrap()))
            .map_err(|_| invalid("Q42 lexicon blob offset exceeds platform"))?;
        if declared_count != count
            || blob_offset != PAGED_PAGE_HEADER_SIZE + count * 16
            || blob_offset > page.len()
        {
            return Err(invalid("Q42 lexicon page is malformed"));
        }
        let mut left = 0usize;
        let mut right = count;
        while left < right {
            let mid = left + (right - left) / 2;
            let index = PAGED_PAGE_HEADER_SIZE + mid * 16;
            let entry_hash = u64::from_le_bytes(page[index..index + 8].try_into().unwrap());
            if entry_hash < hash {
                left = mid + 1;
                continue;
            }
            if entry_hash > hash {
                right = mid;
                continue;
            }
            let relative = usize::try_from(u64::from_le_bytes(
                page[index + 8..index + 16].try_into().unwrap(),
            ))
            .map_err(|_| invalid("Q42 lexicon string offset exceeds platform"))?;
            let start = blob_offset
                .checked_add(relative)
                .ok_or_else(|| invalid("Q42 lexicon string offset overflow"))?;
            if start + 3 > page.len() || page[start] != 1 {
                return Err(invalid("Q42 lexicon string entry is malformed"));
            }
            let length =
                u16::from_le_bytes(page[start + 1..start + 3].try_into().unwrap()) as usize;
            let end = start
                .checked_add(3 + length)
                .ok_or_else(|| invalid("Q42 lexicon string length overflow"))?;
            if end > page.len() {
                return Err(invalid("Q42 lexicon string extends beyond page"));
            }
            if length > out.len() {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "Q42 lexicon output buffer is too small",
                ));
            }
            std::str::from_utf8(&page[start + 3..end])
                .map_err(|_| invalid("Q42 lexicon string is not UTF-8"))?;
            out[..length].copy_from_slice(&page[start + 3..end]);
            return Ok(Some(length));
        }
        Ok(None)
    }
    pub fn read_bidx_into(&self, out: &mut [u8]) -> io::Result<()> {
        self.read_section(self.header.bidx_offset, self.header.bidx_length, out)
    }

    /// Return the size of the front-embedded logical-volume manifest, if this
    /// segment is a root. The caller can use [`Self::read_volume_manifest_into`]
    /// to retrieve exactly those bytes.
    pub fn volume_manifest_length(&self) -> io::Result<Option<usize>> {
        let Some((offset, length)) = self.header.volume_manifest_range() else {
            return Ok(None);
        };
        let length =
            usize::try_from(length).map_err(|_| invalid("Q42 manifest exceeds platform"))?;
        if length == 0 || length > super::manifest::MAX_VOLUME_MANIFEST_BYTES {
            return Err(invalid(
                "Q42 root has an invalid embedded volume manifest length",
            ));
        }
        Q42ByteRange { offset, length }.validate_for(self.source_length)?;
        Ok(Some(length))
    }

    pub fn read_volume_manifest_into(&self, out: &mut [u8]) -> io::Result<bool> {
        let Some(length) = self.volume_manifest_length()? else {
            return Ok(false);
        };
        if out.len() != length {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 manifest output buffer has wrong length",
            ));
        }
        let (offset, _) = self
            .header
            .volume_manifest_range()
            .expect("manifest length was present");
        self.source
            .read_range_into(Q42ByteRange { offset, length }, out)?;
        Ok(true)
    }
    fn read_section(&self, offset: u64, length: u64, out: &mut [u8]) -> io::Result<()> {
        let length =
            usize::try_from(length).map_err(|_| invalid("Q42 section exceeds platform"))?;
        if out.len() != length {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 section output buffer has wrong length",
            ));
        }
        self.source
            .read_range_into(Q42ByteRange { offset, length }, out)
    }

    pub fn block_directory_entry(&self, index: usize) -> io::Result<BlockDirectoryEntry> {
        if index >= self.header.block_count as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 block index out of range",
            ));
        }
        let offset = self
            .header
            .block_dir_offset
            .checked_add((index * BlockDirectoryEntry::SIZE) as u64)
            .ok_or_else(|| invalid("Q42 directory offset overflow"))?;
        let mut bytes = [0u8; BlockDirectoryEntry::SIZE];
        self.source.read_range_into(
            Q42ByteRange {
                offset,
                length: BlockDirectoryEntry::SIZE,
            },
            &mut bytes,
        )?;
        Ok(BlockDirectoryEntry::from_bytes(&bytes))
    }

    /// Object-hash bounds read from the first and last BIDX entry. This avoids
    /// materialising the index merely to validate a manifest segment.
    pub fn object_hash_bounds(&self) -> io::Result<Option<(u64, u64)>> {
        let count = self.bidx_block_count()?;
        if count == 0 {
            return Ok(None);
        }
        let first = self.bidx_entry(0)?;
        let last = self.bidx_entry(count - 1)?;
        if first.0 > first.1 || last.0 > last.1 || first.0 > last.1 {
            return Err(invalid("Q42 BIDX object bounds are invalid"));
        }
        Ok(Some((first.0, last.1)))
    }

    /// Return the complete BIDX interval which can contain an object hash.
    /// Each comparison fetches one fixed 16-byte BIDX entry; there is no index
    /// allocation or whole-index transfer.
    pub fn bidx_block_range_for_hash(
        &self,
        object_hash: u64,
    ) -> io::Result<Option<BidxBlockRange>> {
        let block_count = self.bidx_block_count()?;
        if block_count == 0 {
            return Ok(None);
        }
        let mut lo = 0usize;
        let mut hi = block_count;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.bidx_entry(mid)?.1 < object_hash {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let start = lo;
        if start == block_count || self.bidx_entry(start)?.0 > object_hash {
            return Ok(None);
        }
        lo = start;
        hi = block_count;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.bidx_entry(mid)?.0 <= object_hash {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        Ok(Some(BidxBlockRange { start, end: lo }))
    }

    /// Fill one bounded page of BIDX block indices. This retains complete
    /// heavy-hitter semantics while forcing callers to provide the cap.
    pub fn bidx_blocks_for_hash_into(
        &self,
        object_hash: u64,
        cursor: usize,
        out: &mut [usize],
    ) -> io::Result<Option<BidxMatchPage>> {
        let Some(range) = self.bidx_block_range_for_hash(object_hash)? else {
            return Ok(None);
        };
        if cursor > range.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 BIDX cursor is beyond the matching interval",
            ));
        }
        if out.is_empty() && cursor < range.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 BIDX page buffer must contain at least one block index",
            ));
        }
        let returned = (range.len() - cursor).min(out.len());
        for (offset, slot) in out.iter_mut().take(returned).enumerate() {
            *slot = range.start + cursor + offset;
        }
        let next = cursor + returned;
        Ok(Some(BidxMatchPage {
            range,
            returned,
            next_cursor: (next < range.len()).then_some(next),
        }))
    }

    /// Find Quins whose object equals `object_hash`, using the BIDX to fetch
    /// only candidate SuperBlocks. `compressed`, `decoded`, and `out` are all
    /// caller-owned. Reuse `cursor` from the returned page until it is `None`.
    pub fn find_object_into(
        &self,
        object_hash: u64,
        cursor: Q42ObjectSearchCursor,
        compressed: &mut [u8],
        decoded: &mut [u8],
        out: &mut [NQuin],
    ) -> io::Result<Option<Q42ObjectMatchPage>> {
        let Some(block_range) = self.bidx_block_range_for_hash(object_hash)? else {
            return Ok(None);
        };
        if cursor.block_offset > block_range.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 object search cursor is beyond the matching block interval",
            ));
        }
        if out.is_empty() && cursor.block_offset < block_range.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 object search output buffer must contain at least one Quin",
            ));
        }

        let mut written = 0usize;
        let mut block_offset = cursor.block_offset;
        let mut quin_offset = cursor.quin_offset;
        while block_offset < block_range.len() && written < out.len() {
            self.read_superblock_into(block_range.start + block_offset, compressed, decoded)?;
            let live = u64::from_le_bytes(decoded[16..24].try_into().unwrap()) as usize;
            if live > QUINS_PER_BLOCK || quin_offset > live {
                return Err(invalid(
                    "Q42 object search encountered an invalid SuperBlock",
                ));
            }
            while quin_offset < live && written < out.len() {
                let offset = SUPERBLOCK_HEADER + quin_offset * QUIN_SIZE;
                let quin =
                    bytemuck::pod_read_unaligned::<NQuin>(&decoded[offset..offset + QUIN_SIZE]);
                if quin.object < object_hash {
                    quin_offset += 1;
                    continue;
                }
                if quin.object > object_hash {
                    quin_offset = live;
                    break;
                }
                out[written] = quin;
                written += 1;
                quin_offset += 1;
            }
            if quin_offset == live {
                block_offset += 1;
                quin_offset = 0;
            }
        }
        let next_cursor = (block_offset < block_range.len()).then_some(Q42ObjectSearchCursor {
            block_offset,
            quin_offset,
        });
        Ok(Some(Q42ObjectMatchPage {
            block_range,
            returned: written,
            next_cursor,
        }))
    }

    /// Fetch and decode one block. `compressed` must fit the directory entry;
    /// `out` must be at least one full decoded SuperBlock.
    pub fn read_superblock_into(
        &self,
        index: usize,
        compressed: &mut [u8],
        out: &mut [u8],
    ) -> io::Result<usize> {
        if out.len() < SUPERBLOCK_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 decoded output buffer is too small",
            ));
        }
        let entry = self.block_directory_entry(index)?;
        let compressed_len = entry.comp_len as usize;
        if compressed_len < 4
            || compressed_len > MAX_COMPRESSED_SUPERBLOCK_SIZE
            || compressed.len() < compressed_len
            || entry.uncomp_len != SUPERBLOCK_SIZE as u32
        {
            return Err(invalid("invalid Q42 compressed block directory entry"));
        }
        let offset = self
            .header
            .data_offset
            .checked_add(entry.rel_offset)
            .ok_or_else(|| invalid("Q42 compressed block offset overflow"))?;
        self.source.read_range_into(
            Q42ByteRange {
                offset,
                length: compressed_len,
            },
            &mut compressed[..compressed_len],
        )?;
        let declared = u32::from_le_bytes(compressed[0..4].try_into().unwrap()) as usize;
        if declared != SUPERBLOCK_SIZE {
            return Err(invalid("Q42 LZ4 prefix does not declare one SuperBlock"));
        }
        let decoded =
            lz4_flex::decompress_into(&compressed[4..compressed_len], &mut out[..declared])
                .map_err(|error| invalid(format!("decode Q42 range block: {error}")))?;
        if decoded != declared {
            return Err(invalid("Q42 range block decoded to an unexpected length"));
        }
        Ok(decoded)
    }

    pub fn into_source(self) -> S {
        self.source
    }

    fn bidx_block_count(&self) -> io::Result<usize> {
        let bidx_length = self.header.bidx_length;
        if bidx_length < BIDX_HEADER_BYTES as u64 {
            return Err(invalid("Q42 BIDX is shorter than its header"));
        }
        let mut bytes = [0u8; BIDX_HEADER_BYTES];
        let offset = self.header.bidx_offset;
        self.source.read_range_into(
            Q42ByteRange {
                offset,
                length: BIDX_HEADER_BYTES,
            },
            &mut bytes,
        )?;
        if bytes[0..4] != BIDX_MAGIC || u32::from_le_bytes(bytes[4..8].try_into().unwrap()) != 1 {
            return Err(invalid("unsupported Q42 BIDX header"));
        }
        let count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        if count != self.header.block_count as usize {
            return Err(invalid("Q42 BIDX count does not match the block directory"));
        }
        let expected = BIDX_HEADER_BYTES
            .checked_add(
                count
                    .checked_mul(BIDX_ENTRY_BYTES)
                    .ok_or_else(|| invalid("Q42 BIDX entry count overflow"))?,
            )
            .ok_or_else(|| invalid("Q42 BIDX length overflow"))?;
        if bidx_length != expected as u64 {
            return Err(invalid("Q42 BIDX length does not match its entry count"));
        }
        Ok(count)
    }

    fn bidx_entry(&self, index: usize) -> io::Result<(u64, u64)> {
        let block_count = usize::try_from(self.header.block_count)
            .map_err(|_| invalid("Q42 block count exceeds platform"))?;
        if index >= block_count {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 BIDX entry index out of range",
            ));
        }
        let offset = self
            .header
            .bidx_offset
            .checked_add(BIDX_HEADER_BYTES as u64)
            .and_then(|value| value.checked_add((index * BIDX_ENTRY_BYTES) as u64))
            .ok_or_else(|| invalid("Q42 BIDX entry offset overflow"))?;
        let mut bytes = [0u8; BIDX_ENTRY_BYTES];
        self.source.read_range_into(
            Q42ByteRange {
                offset,
                length: BIDX_ENTRY_BYTES,
            },
            &mut bytes,
        )?;
        let min = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let max = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        if min > max {
            return Err(invalid("Q42 BIDX entry has min > max"));
        }
        Ok((min, max))
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::{
        write_unified_volume, write_volume_root, Q42RangeVolumeSet, Q42VolumeManifest,
    };
    use super::*;
    use crate::mini_parser::hash_token;
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;
    use crate::NQuin;
    use std::collections::HashMap;
    use tempfile::NamedTempFile;

    fn sample_volume() -> (NamedTempFile, NQuin) {
        let subject = hash_token("urn:q42:range-subject");
        let predicate = hash_token("urn:q42:range-predicate");
        let object = hash_token("urn:q42:range-object");
        let quin = NQuin {
            subject,
            predicate,
            object,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let mut lex = HashMap::new();
        lex.insert(subject, "urn:q42:range-subject".to_string());
        lex.insert(predicate, "urn:q42:range-predicate".to_string());
        lex.insert(object, "urn:q42:range-object".to_string());
        let file = NamedTempFile::new().unwrap();
        write_unified_volume(file.path(), &lex, &[(object, object)], &[vec![quin]]).unwrap();
        (file, quin)
    }

    #[test]
    fn range_volume_reads_only_the_directory_entry_and_block() {
        let (file, quin) = sample_volume();
        let source = super::super::range::LocalFileRangeSource::open(file.path()).unwrap();
        let volume = Q42RangeVolume::open(source).unwrap();
        assert_eq!(volume.block_count(), 1);
        let entry = volume.block_directory_entry(0).unwrap();
        assert!(entry.comp_len as usize <= MAX_COMPRESSED_SUPERBLOCK_SIZE);

        let mut compressed = [0u8; MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = [0u8; SUPERBLOCK_SIZE];
        assert_eq!(
            volume
                .read_superblock_into(0, &mut compressed, &mut decoded)
                .unwrap(),
            SUPERBLOCK_SIZE
        );
        assert_eq!(u64::from_le_bytes(decoded[16..24].try_into().unwrap()), 1);
        assert_eq!(
            u64::from_le_bytes(decoded[160..168].try_into().unwrap()),
            quin.subject
        );
    }

    #[test]
    fn range_volume_block_read_is_zero_heap() {
        let (file, _) = sample_volume();
        let source = super::super::range::LocalFileRangeSource::open(file.path()).unwrap();
        let volume = Q42RangeVolume::open(source).unwrap();
        let mut compressed = [0u8; MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = [0u8; SUPERBLOCK_SIZE];
        assert_zero_alloc("q42_range_volume_block_read", || {
            volume
                .read_superblock_into(0, &mut compressed, &mut decoded)
                .unwrap();
        });
    }

    #[test]
    fn range_volume_resolves_one_paged_lexicon_page_without_heap() {
        let (file, quin) = sample_volume();
        let source = super::super::range::LocalFileRangeSource::open(file.path()).unwrap();
        let volume = Q42RangeVolume::open(source).unwrap();
        let mut page = [0u8; 4_096];
        let mut text = [0u8; 128];
        let length = volume
            .lookup_lexicon_hash_into(quin.object, &mut page, &mut text)
            .unwrap()
            .unwrap();
        assert_eq!(&text[..length], b"urn:q42:range-object");
        assert_eq!(
            volume
                .lookup_lexicon_hash_into(7, &mut page, &mut text)
                .unwrap(),
            None
        );
        assert_zero_alloc("q42_range_volume_paged_lex_lookup", || {
            volume
                .lookup_lexicon_hash_into(quin.object, &mut page, &mut text)
                .unwrap();
        });
    }

    #[test]
    fn range_volume_bidx_pages_complete_heavy_hitters() {
        let (file, quin) = sample_volume();
        let mut lex = HashMap::new();
        lex.insert(quin.subject, "urn:q42:range-subject".to_string());
        lex.insert(quin.predicate, "urn:q42:range-predicate".to_string());
        lex.insert(quin.object, "urn:q42:range-object".to_string());
        write_unified_volume(
            file.path(),
            &lex,
            &[(quin.object, quin.object); 5],
            &[vec![quin], vec![quin], vec![quin], vec![quin], vec![quin]],
        )
        .unwrap();
        let source = super::super::range::LocalFileRangeSource::open(file.path()).unwrap();
        let volume = Q42RangeVolume::open(source).unwrap();
        let mut page = [usize::MAX; 2];
        let first = volume
            .bidx_blocks_for_hash_into(quin.object, 0, &mut page)
            .unwrap()
            .unwrap();
        assert_eq!(first.range, BidxBlockRange { start: 0, end: 5 });
        assert_eq!(&page, &[0, 1]);
        let last = volume
            .bidx_blocks_for_hash_into(quin.object, first.next_cursor.unwrap(), &mut page)
            .unwrap()
            .unwrap();
        assert_eq!(&page, &[2, 3]);
        assert_eq!(last.next_cursor, Some(4));
    }

    #[test]
    fn range_volume_object_search_is_paged_and_zero_heap() {
        let (file, quin) = sample_volume();
        let mut lex = HashMap::new();
        lex.insert(quin.subject, "urn:q42:range-subject".to_string());
        lex.insert(quin.predicate, "urn:q42:range-predicate".to_string());
        lex.insert(quin.object, "urn:q42:range-object".to_string());
        write_unified_volume(
            file.path(),
            &lex,
            &[(quin.object, quin.object); 3],
            &[vec![quin], vec![quin], vec![quin]],
        )
        .unwrap();
        let source = super::super::range::LocalFileRangeSource::open(file.path()).unwrap();
        let volume = Q42RangeVolume::open(source).unwrap();
        let mut compressed = [0u8; MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = [0u8; SUPERBLOCK_SIZE];
        let mut out = [NQuin::default(); 2];
        let first = volume
            .find_object_into(
                quin.object,
                Q42ObjectSearchCursor::default(),
                &mut compressed,
                &mut decoded,
                &mut out,
            )
            .unwrap()
            .unwrap();
        assert_eq!(first.returned, 2);
        assert_eq!(out, [quin, quin]);
        let second = volume
            .find_object_into(
                quin.object,
                first.next_cursor.unwrap(),
                &mut compressed,
                &mut decoded,
                &mut out,
            )
            .unwrap()
            .unwrap();
        assert_eq!(second.returned, 1);
        assert_eq!(out[0], quin);
        assert_eq!(second.next_cursor, None);

        assert_zero_alloc("q42_range_volume_object_search", || {
            volume
                .find_object_into(
                    quin.object,
                    Q42ObjectSearchCursor::default(),
                    &mut compressed,
                    &mut decoded,
                    &mut out,
                )
                .unwrap();
        });
    }

    #[test]
    fn range_query_planner_uses_bidx_and_pages_matching_quins() {
        let (file, quin) = sample_volume();
        let mut lex = HashMap::new();
        lex.insert(quin.subject, "urn:q42:range-subject".to_string());
        lex.insert(quin.predicate, "urn:q42:range-predicate".to_string());
        lex.insert(quin.object, "urn:q42:range-object".to_string());
        write_unified_volume(
            file.path(),
            &lex,
            &[(quin.object, quin.object); 3],
            &[vec![quin], vec![quin], vec![quin]],
        )
        .unwrap();
        let source = super::super::range::LocalFileRangeSource::open(file.path()).unwrap();
        let volume = Q42RangeVolume::open(source).unwrap();
        let plan = Q42RangeQueryPlan::for_pattern(Q42RangeQueryPattern {
            object: Some(quin.object),
            predicate: Some(quin.predicate),
            ..Default::default()
        });
        assert_eq!(plan.strategy, Q42RangeQueryStrategy::ObjectBidx);
        let mut compressed = [0u8; MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = [0u8; SUPERBLOCK_SIZE];
        let mut out = [NQuin::default(); 2];
        let first = volume
            .execute_query_page_into(
                plan,
                Q42RangeQueryCursor::default(),
                &mut compressed,
                &mut decoded,
                &mut out,
            )
            .unwrap();
        assert_eq!(first.returned, 2);
        assert_eq!(out, [quin, quin]);
        let second = volume
            .execute_query_page_into(
                plan,
                first.next_cursor.unwrap(),
                &mut compressed,
                &mut decoded,
                &mut out,
            )
            .unwrap();
        assert_eq!(second.returned, 1);
        assert_eq!(second.next_cursor, None);
        assert_zero_alloc("q42_range_query_bidx_page", || {
            volume
                .execute_query_page_into(
                    plan,
                    Q42RangeQueryCursor::default(),
                    &mut compressed,
                    &mut decoded,
                    &mut out,
                )
                .unwrap();
        });
    }

    #[test]
    fn range_volume_set_opens_front_embedded_root_and_verifies_segments() {
        let dir = tempfile::TempDir::new().unwrap();
        let root_path = dir.path().join("root.q42");
        let first_path = dir.path().join("segment-000.q42");
        let second_path = dir.path().join("segment-001.q42");
        let (first_quin, first_lex) = sample_quin("urn:q42:one");
        let (second_quin, second_lex) = sample_quin("urn:q42:two");
        let mut entries = [(first_quin, first_lex), (second_quin, second_lex)];
        entries.sort_unstable_by_key(|(quin, _)| quin.object);
        write_unified_volume(
            &first_path,
            &entries[0].1,
            &[(entries[0].0.object, entries[0].0.object)],
            &[vec![entries[0].0]],
        )
        .unwrap();
        write_unified_volume(
            &second_path,
            &entries[1].1,
            &[(entries[1].0.object, entries[1].0.object)],
            &[vec![entries[1].0]],
        )
        .unwrap();
        let manifest = Q42VolumeManifest {
            generation: 1,
            segments: vec![
                Q42VolumeManifest::segment_from_file(&first_path, "segment-000.q42".into())
                    .unwrap(),
                Q42VolumeManifest::segment_from_file(&second_path, "segment-001.q42".into())
                    .unwrap(),
            ],
        };
        write_volume_root(&root_path, &manifest).unwrap();

        let root_source = super::super::range::LocalFileRangeSource::open(&root_path).unwrap();
        let root = Q42RangeVolume::open(root_source).unwrap();
        let factory = |entry: &super::super::manifest::Q42VolumeSegment| {
            super::super::range::LocalFileRangeSource::open(&dir.path().join(&entry.locator))
        };
        let set = Q42RangeVolumeSet::open_root(&root, &factory).unwrap();
        assert_eq!(set.segment_index_for_object(entries[0].0.object), Some(0));
        assert_eq!(set.segment_index_for_object(entries[1].0.object), Some(1));
        let mut digest_scratch = [0u8; 1024];
        set.verify_segment_hashes(&mut digest_scratch).unwrap();
        let mut compressed = [0u8; MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = [0u8; SUPERBLOCK_SIZE];
        set.verify_segment_quin_counts(&mut compressed, &mut decoded)
            .unwrap();
        let plan = Q42RangeQueryPlan::for_pattern(Q42RangeQueryPattern {
            predicate: Some(entries[0].0.predicate),
            ..Default::default()
        });
        let mut out = [NQuin::default(); 2];
        let page = set
            .execute_query_page_into(
                plan,
                super::super::manifest::Q42VolumeSetQueryCursor::default(),
                &mut compressed,
                &mut decoded,
                &mut out,
            )
            .unwrap();
        assert_eq!(page.returned, 2);
        assert_eq!(page.next_cursor, None);
    }

    fn sample_quin(object_text: &str) -> (NQuin, HashMap<u64, String>) {
        let subject = hash_token("urn:q42:range-subject");
        let predicate = hash_token("urn:q42:range-predicate");
        let object = hash_token(object_text);
        let quin = NQuin {
            subject,
            predicate,
            object,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let mut lex = HashMap::new();
        lex.insert(subject, "urn:q42:range-subject".to_string());
        lex.insert(predicate, "urn:q42:range-predicate".to_string());
        lex.insert(object, object_text.to_string());
        (quin, lex)
    }
}
