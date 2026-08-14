//! Read `.q42.lex` reverse-lexicon sidecars (Q42LEX format from qualia-cli ingest).

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use memmap2::Mmap;

pub const LEX_MAGIC: [u8; 8] = *b"Q42LEX\0\0";
const MAGIC: &[u8; 8] = &LEX_MAGIC;
pub const LEX_HEADER_SIZE: usize = 32;
const HEADER_SIZE: usize = LEX_HEADER_SIZE;
const INDEX_ENTRY_SIZE: usize = 16;
pub const LEX_VERSION_PAGED: u64 = 2;
pub const PAGED_DIRECTORY_HEADER_SIZE: usize = 8;
pub const PAGED_DIRECTORY_ENTRY_SIZE: usize = 32;
pub const PAGED_PAGE_HEADER_SIZE: usize = 16;
/// Fixed upper bound used by the canonical writer.  Pages are independently
/// addressable, so an HTTP/IPFS reader only needs this many dictionary records
/// (plus their strings) for one hash lookup.
pub const DEFAULT_LEX_PAGE_ENTRIES: usize = 4_096;

/// Type tags for lexicon entries (1-byte prefix in payload)
const LEX_TAG_STRING: u8 = 0x01; // UTF-8 string
const LEX_TAG_EMBEDDED: u8 = 0x02; // Embedded triple [u64; 3]
const LEX_TAG_WEBIZEN: u8 = 0x03; // Authoritative Webizen identity

/// Serialize a hash → string map into the canonical `Q42LEX` byte layout that [`Q42LexMmap`] reads
/// back (magic, sorted 16-byte index, tagged string blob). This is the **write** side of the lexicon:
/// the RDF ingest calls it in lossless ("Complete") mode so every subject/predicate URI and every
/// literal is recoverable via [`Q42LexMmap::lookup_hash`], instead of being hashed away.
///
/// Strings are stored as **UTF-8** (`LEX_TAG_STRING`), preserving the full Unicode range — this is
/// essential, not cosmetic: a lexicon that only kept ASCII would silently erase most of the world's
/// languages (WordNet alone carries Finnish, Thai, and dozens more). A string longer than the 16-bit
/// length field can express is truncated at a UTF-8 **character boundary** (never mid-codepoint), so a
/// multi-byte grapheme is never split into invalid bytes.
///
/// The map is keyed by the same value stored in the quin field (for objects that is the
/// `OBJECT_HASH_MASK`-masked hash), so `lookup_hash(quin.object)` resolves directly. Entries are sorted
/// by hash for the reader's binary search; the caller's map has already de-duplicated by hash.
/// Returns [`LexError::TermTooLong`] rather than silently truncating a term.
pub fn serialize_string_lexicon(entries: &HashMap<u64, String>) -> Result<Vec<u8>, LexError> {
    let mut sorted: Vec<(&u64, &String)> = entries.iter().collect();
    sorted.sort_unstable_by_key(|(h, _)| **h);
    let entry_count = sorted.len() as u64;
    let strings_offset = HEADER_SIZE as u64 + entry_count * INDEX_ENTRY_SIZE as u64;

    let mut index: Vec<u8> = Vec::with_capacity(sorted.len() * INDEX_ENTRY_SIZE);
    let mut blob: Vec<u8> = Vec::new();
    for (hash, text) in &sorted {
        let str_off = blob.len() as u64;
        if text.len() > u16::MAX as usize {
            return Err(LexError::TermTooLong);
        }
        blob.push(LEX_TAG_STRING);
        blob.extend_from_slice(&(text.len() as u16).to_le_bytes());
        blob.extend_from_slice(text.as_bytes());
        index.extend_from_slice(&hash.to_le_bytes());
        index.extend_from_slice(&str_off.to_le_bytes());
    }

    let mut out = Vec::with_capacity(HEADER_SIZE + index.len() + blob.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&entry_count.to_le_bytes());
    out.extend_from_slice(&strings_offset.to_le_bytes());
    out.extend_from_slice(&1u64.to_le_bytes()); // format version
    out.extend_from_slice(&index);
    out.extend_from_slice(&blob);
    Ok(out)
}

/// Serialize the v2 paged Q42LEX representation.
///
/// Unlike the original monolithic index, this stores a small page directory at
/// the front followed by independently valid pages.  Pages deliberately stay
/// uncompressed in v2: they can be returned as borrowed `&str` values by the
/// existing zero-allocation resolver.  Compression is a transport concern for
/// a whole Q42 range response; keeping page contents directly addressable is
/// what preserves the ABI and HTTP-range streamability today.
pub fn serialize_paged_string_lexicon(
    entries: &HashMap<u64, String>,
    page_entries: usize,
) -> Result<Vec<u8>, LexError> {
    if page_entries == 0 {
        return Err(LexError::BadIndex);
    }
    let mut sorted: Vec<(&u64, &String)> = entries.iter().collect();
    sorted.sort_unstable_by_key(|(hash, _)| **hash);
    let page_count = (sorted.len() + page_entries - 1) / page_entries;
    let directory_len = PAGED_DIRECTORY_HEADER_SIZE
        .checked_add(
            page_count
                .checked_mul(PAGED_DIRECTORY_ENTRY_SIZE)
                .ok_or(LexError::Truncated)?,
        )
        .ok_or(LexError::Truncated)?;
    let mut out = Vec::with_capacity(HEADER_SIZE + directory_len + entries.len() * 24);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(sorted.len() as u64).to_le_bytes());
    out.extend_from_slice(&(HEADER_SIZE as u64).to_le_bytes());
    out.extend_from_slice(&LEX_VERSION_PAGED.to_le_bytes());
    out.extend_from_slice(&(page_count as u64).to_le_bytes());
    out.resize(HEADER_SIZE + directory_len, 0);

    for (page_index, chunk) in sorted.chunks(page_entries).enumerate() {
        let page_offset = out.len() as u64;
        let page_start = out.len();
        out.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        let blob_offset = PAGED_PAGE_HEADER_SIZE
            .checked_add(
                chunk
                    .len()
                    .checked_mul(INDEX_ENTRY_SIZE)
                    .ok_or(LexError::Truncated)?,
            )
            .ok_or(LexError::Truncated)?;
        out.extend_from_slice(&(blob_offset as u64).to_le_bytes());
        let index_start = out.len();
        out.resize(index_start + chunk.len() * INDEX_ENTRY_SIZE, 0);
        for (entry_index, (hash, text)) in chunk.iter().enumerate() {
            if text.len() > u16::MAX as usize {
                return Err(LexError::TermTooLong);
            }
            let relative = (out.len() - page_start - blob_offset) as u64;
            out.push(LEX_TAG_STRING);
            out.extend_from_slice(&(text.len() as u16).to_le_bytes());
            out.extend_from_slice(text.as_bytes());
            let index_offset = index_start + entry_index * INDEX_ENTRY_SIZE;
            out[index_offset..index_offset + 8].copy_from_slice(&hash.to_le_bytes());
            out[index_offset + 8..index_offset + 16].copy_from_slice(&relative.to_le_bytes());
        }
        let page_length = (out.len() - page_start) as u64;
        let directory =
            HEADER_SIZE + PAGED_DIRECTORY_HEADER_SIZE + page_index * PAGED_DIRECTORY_ENTRY_SIZE;
        out[directory..directory + 8].copy_from_slice(&chunk[0].0.to_le_bytes());
        out[directory + 8..directory + 16].copy_from_slice(&page_offset.to_le_bytes());
        out[directory + 16..directory + 24].copy_from_slice(&page_length.to_le_bytes());
        out[directory + 24..directory + 28].copy_from_slice(&(chunk.len() as u32).to_le_bytes());
    }
    Ok(out)
}

/// Largest UTF-8 prefix of `s` that fits in `max_bytes`, never splitting a codepoint.
/// Zero-allocation lexicon key for in-memory lookups
pub enum LexiconKey<'a> {
    /// UTF-8 string reference
    Str(&'a str),
    /// Embedded triple reference
    Triple(&'a [u64; 3]),
}

/// Lexicon entry payload for serialization
#[derive(Debug, Clone)]
pub enum LexiconEntry {
    /// UTF-8 string
    String(String),
    /// Embedded triple [subject, predicate, object]
    EmbeddedTriple([u64; 3]),
    /// Webizen identity (future implementation)
    Webizen(String),
}

/// In-memory hash → UTF-8 string map from a `.q42.lex` file (cold-path loader).
#[derive(Debug, Default)]
pub struct Q42Lexicon {
    pub entries: HashMap<u64, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexError {
    InvalidMagic,
    Truncated,
    BadStringOffset,
    BadIndex,
    BadEntry,
    InvalidUtf8,
    TermTooLong,
}

/// Zero-allocation view over a memory-mapped `.q42.lex` slice (sorted hash index).
#[derive(Debug, Clone, Copy)]
pub struct Q42LexMmap<'a> {
    data: &'a [u8],
    entry_count: usize,
    strings_offset: usize,
    format_version: u64,
    page_count: usize,
}

impl<'a> Q42LexMmap<'a> {
    /// Parse a Q42LEX byte slice (typically from `mmap`).
    pub fn from_bytes(data: &'a [u8]) -> Result<Self, LexError> {
        if data.len() < HEADER_SIZE {
            return Err(LexError::Truncated);
        }
        if data[0..8] != *MAGIC {
            return Err(LexError::InvalidMagic);
        }
        let format_version = u64::from_le_bytes(data[24..32].try_into().unwrap());
        let entry_count = usize::try_from(u64::from_le_bytes(data[8..16].try_into().unwrap()))
            .map_err(|_| LexError::Truncated)?;
        let strings_offset = usize::try_from(u64::from_le_bytes(data[16..24].try_into().unwrap()))
            .map_err(|_| LexError::Truncated)?;
        let index_end = HEADER_SIZE
            .checked_add(
                entry_count
                    .checked_mul(INDEX_ENTRY_SIZE)
                    .ok_or(LexError::Truncated)?,
            )
            .ok_or(LexError::Truncated)?;
        let (page_count, flat) = match format_version {
            1 => (0, true),
            LEX_VERSION_PAGED => {
                let page_count_end = strings_offset
                    .checked_add(PAGED_DIRECTORY_HEADER_SIZE)
                    .ok_or(LexError::Truncated)?;
                if page_count_end > data.len() {
                    return Err(LexError::Truncated);
                }
                let page_count = usize::try_from(u64::from_le_bytes(
                    data[strings_offset..page_count_end].try_into().unwrap(),
                ))
                .map_err(|_| LexError::Truncated)?;
                let directory_end = page_count_end
                    .checked_add(
                        page_count
                            .checked_mul(PAGED_DIRECTORY_ENTRY_SIZE)
                            .ok_or(LexError::Truncated)?,
                    )
                    .ok_or(LexError::Truncated)?;
                if strings_offset != HEADER_SIZE || directory_end > data.len() {
                    return Err(LexError::Truncated);
                }
                (page_count, false)
            }
            _ => return Err(LexError::BadIndex),
        };
        if flat && (index_end > data.len() || strings_offset != index_end) {
            return Err(LexError::Truncated);
        }
        let view = Self {
            data,
            entry_count,
            strings_offset,
            format_version,
            page_count,
        };
        view.validate_entries()?;
        Ok(view)
    }

    #[inline]
    pub fn entry_count(&self) -> usize {
        self.entry_count
    }

    /// Hash at sorted ordinal `i`, independent of whether the lexicon is the
    /// legacy flat layout or the paged v2 layout. Cold loaders use this rather
    /// than reaching into an on-disk index with v1 assumptions.
    pub fn hash_at(&self, i: usize) -> Option<u64> {
        if i >= self.entry_count {
            return None;
        }
        if self.format_version == LEX_VERSION_PAGED {
            let mut base = 0usize;
            for page in 0..self.page_count {
                let (_, offset, _, count) = self.page_directory_entry(page)?;
                if i < base + count {
                    let entry = offset + PAGED_PAGE_HEADER_SIZE + (i - base) * INDEX_ENTRY_SIZE;
                    return Some(u64::from_le_bytes(
                        self.data.get(entry..entry + 8)?.try_into().ok()?,
                    ));
                }
                base += count;
            }
            return None;
        }
        let off = HEADER_SIZE + i * INDEX_ENTRY_SIZE;
        Some(u64::from_le_bytes(
            self.data.get(off..off + 8)?.try_into().ok()?,
        ))
    }

    /// Binary search for `hash` in the sorted index; returns the UTF-8 lexeme slice.
    pub fn lookup_hash(&self, hash: u64) -> Option<&'a str> {
        if self.format_version == LEX_VERSION_PAGED {
            return self.lookup_hash_paged(hash);
        }
        let mut lo = 0usize;
        let mut hi = self.entry_count;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let off = HEADER_SIZE + mid * INDEX_ENTRY_SIZE;
            let entry_hash = u64::from_le_bytes(self.data[off..off + 8].try_into().ok()?);
            match entry_hash.cmp(&hash) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => {
                    let str_off =
                        u64::from_le_bytes(self.data[off + 8..off + 16].try_into().ok()?) as usize;
                    return Self::read_string_at(self.data, self.strings_offset, str_off);
                }
            }
        }
        None
    }

    /// The lexeme string of the `i`-th index entry (`0..entry_count`), if it is a UTF-8 string entry
    /// (not an embedded-triple / Webizen entry). Enables iterating ALL lexicon strings without knowing
    /// their hashes — e.g. to assemble a calibration corpus from a WordNet q42's gloss text.
    pub fn string_at(&self, i: usize) -> Option<&'a str> {
        if i >= self.entry_count {
            return None;
        }
        if self.format_version == LEX_VERSION_PAGED {
            let mut base = 0usize;
            for page in 0..self.page_count {
                let (_, offset, length, count) = self.page_directory_entry(page)?;
                if i < base + count {
                    return self.page_string_at(offset, length, i - base);
                }
                base += count;
            }
            return None;
        }
        let off = HEADER_SIZE + i * INDEX_ENTRY_SIZE;
        let str_off = u64::from_le_bytes(self.data[off + 8..off + 16].try_into().ok()?) as usize;
        Self::read_string_at(self.data, self.strings_offset, str_off)
    }

    /// Validate every index entry and its tagged payload while the full byte
    /// slice is available. This keeps corrupt lexicon sections from becoming
    /// deferred `None` values during query execution.
    fn validate_entries(&self) -> Result<(), LexError> {
        if self.format_version == LEX_VERSION_PAGED {
            return self.validate_paged_entries();
        }
        let mut previous_hash = None;
        for i in 0..self.entry_count {
            let off = HEADER_SIZE + i * INDEX_ENTRY_SIZE;
            let hash = u64::from_le_bytes(
                self.data[off..off + 8]
                    .try_into()
                    .map_err(|_| LexError::BadIndex)?,
            );
            if let Some(previous) = previous_hash {
                if hash <= previous {
                    return Err(LexError::BadIndex);
                }
            }
            previous_hash = Some(hash);
            let rel_off = usize::try_from(u64::from_le_bytes(
                self.data[off + 8..off + 16]
                    .try_into()
                    .map_err(|_| LexError::BadIndex)?,
            ))
            .map_err(|_| LexError::BadStringOffset)?;
            let start = self
                .strings_offset
                .checked_add(rel_off)
                .ok_or(LexError::BadStringOffset)?;
            let tag = *self.data.get(start).ok_or(LexError::BadStringOffset)?;
            match tag {
                LEX_TAG_STRING | LEX_TAG_WEBIZEN => {
                    let length_end = start.checked_add(3).ok_or(LexError::BadEntry)?;
                    let length_bytes = self
                        .data
                        .get(start + 1..length_end)
                        .ok_or(LexError::BadEntry)?;
                    let length = u16::from_le_bytes(
                        length_bytes.try_into().map_err(|_| LexError::BadEntry)?,
                    ) as usize;
                    let text_start = length_end;
                    let text_end = text_start.checked_add(length).ok_or(LexError::BadEntry)?;
                    let text = self
                        .data
                        .get(text_start..text_end)
                        .ok_or(LexError::BadEntry)?;
                    std::str::from_utf8(text).map_err(|_| LexError::InvalidUtf8)?;
                }
                LEX_TAG_EMBEDDED => {
                    let end = start.checked_add(25).ok_or(LexError::BadEntry)?;
                    self.data.get(start..end).ok_or(LexError::BadEntry)?;
                }
                _ => return Err(LexError::BadEntry),
            }
        }
        Ok(())
    }

    fn page_directory_entry(&self, index: usize) -> Option<(u64, usize, usize, usize)> {
        if index >= self.page_count {
            return None;
        }
        let offset =
            self.strings_offset + PAGED_DIRECTORY_HEADER_SIZE + index * PAGED_DIRECTORY_ENTRY_SIZE;
        let first_hash = u64::from_le_bytes(self.data.get(offset..offset + 8)?.try_into().ok()?);
        let page_offset = usize::try_from(u64::from_le_bytes(
            self.data.get(offset + 8..offset + 16)?.try_into().ok()?,
        ))
        .ok()?;
        let page_length = usize::try_from(u64::from_le_bytes(
            self.data.get(offset + 16..offset + 24)?.try_into().ok()?,
        ))
        .ok()?;
        let count =
            u32::from_le_bytes(self.data.get(offset + 24..offset + 28)?.try_into().ok()?) as usize;
        Some((first_hash, page_offset, page_length, count))
    }

    fn lookup_hash_paged(&self, hash: u64) -> Option<&'a str> {
        let mut lo = 0usize;
        let mut hi = self.page_count;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.page_directory_entry(mid)?.0 <= hash {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let page = lo.checked_sub(1)?;
        let (_, offset, length, count) = self.page_directory_entry(page)?;
        let mut left = 0usize;
        let mut right = count;
        while left < right {
            let mid = left + (right - left) / 2;
            let entry = offset + PAGED_PAGE_HEADER_SIZE + mid * INDEX_ENTRY_SIZE;
            let entry_hash = u64::from_le_bytes(self.data.get(entry..entry + 8)?.try_into().ok()?);
            match entry_hash.cmp(&hash) {
                std::cmp::Ordering::Less => left = mid + 1,
                std::cmp::Ordering::Greater => right = mid,
                std::cmp::Ordering::Equal => return self.page_string_at(offset, length, mid),
            }
        }
        None
    }

    fn page_string_at(
        &self,
        page_offset: usize,
        page_length: usize,
        index: usize,
    ) -> Option<&'a str> {
        let page_end = page_offset.checked_add(page_length)?;
        let count = u32::from_le_bytes(
            self.data
                .get(page_offset..page_offset + 4)?
                .try_into()
                .ok()?,
        ) as usize;
        let blob_offset = usize::try_from(u64::from_le_bytes(
            self.data
                .get(page_offset + 8..page_offset + 16)?
                .try_into()
                .ok()?,
        ))
        .ok()?;
        if index >= count {
            return None;
        }
        let entry = page_offset + PAGED_PAGE_HEADER_SIZE + index * INDEX_ENTRY_SIZE;
        let relative = usize::try_from(u64::from_le_bytes(
            self.data.get(entry + 8..entry + 16)?.try_into().ok()?,
        ))
        .ok()?;
        let start = page_offset
            .checked_add(blob_offset)?
            .checked_add(relative)?;
        if start + 3 > page_end || self.data[start] != LEX_TAG_STRING {
            return None;
        }
        let length = u16::from_le_bytes(self.data[start + 1..start + 3].try_into().ok()?) as usize;
        let end = start.checked_add(3)?.checked_add(length)?;
        if end > page_end {
            return None;
        }
        std::str::from_utf8(&self.data[start + 3..end]).ok()
    }

    fn validate_paged_entries(&self) -> Result<(), LexError> {
        let mut total = 0usize;
        let mut previous = None;
        for page in 0..self.page_count {
            let (first, offset, length, count) =
                self.page_directory_entry(page).ok_or(LexError::BadIndex)?;
            if count == 0
                || offset < HEADER_SIZE
                || offset
                    .checked_add(length)
                    .is_none_or(|end| end > self.data.len())
            {
                return Err(LexError::BadIndex);
            }
            if previous.is_some_and(|value| first <= value) {
                return Err(LexError::BadIndex);
            }
            let actual_count =
                u32::from_le_bytes(self.data[offset..offset + 4].try_into().unwrap()) as usize;
            let blob = usize::try_from(u64::from_le_bytes(
                self.data[offset + 8..offset + 16].try_into().unwrap(),
            ))
            .map_err(|_| LexError::BadIndex)?;
            if actual_count != count
                || blob != PAGED_PAGE_HEADER_SIZE + count * INDEX_ENTRY_SIZE
                || blob > length
            {
                return Err(LexError::BadIndex);
            }
            for item in 0..count {
                let entry = offset + PAGED_PAGE_HEADER_SIZE + item * INDEX_ENTRY_SIZE;
                let hash = u64::from_le_bytes(self.data[entry..entry + 8].try_into().unwrap());
                if item == 0 && hash != first {
                    return Err(LexError::BadIndex);
                }
                if previous.is_some_and(|value| hash <= value) {
                    return Err(LexError::BadIndex);
                }
                previous = Some(hash);
                if self.page_string_at(offset, length, item).is_none() {
                    return Err(LexError::BadEntry);
                }
            }
            total = total.checked_add(count).ok_or(LexError::BadIndex)?;
        }
        if total != self.entry_count {
            return Err(LexError::BadIndex);
        }
        Ok(())
    }

    fn read_string_at(data: &[u8], blob_base: usize, rel_off: usize) -> Option<&str> {
        let start = blob_base.checked_add(rel_off)?;
        if start.checked_add(3)? > data.len() {
            return None;
        }
        // Check type tag
        if data[start] != LEX_TAG_STRING {
            return None;
        }
        let len = u16::from_le_bytes(data[start + 1..start + 3].try_into().ok()?) as usize;
        let text_start = start + 3;
        let text_end = text_start.checked_add(len)?;
        if text_end > data.len() {
            return None;
        }
        std::str::from_utf8(&data[text_start..text_end]).ok()
    }

    /// Binary search for `hash` in the sorted index; returns the embedded triple [subject, predicate, object].
    ///
    /// Used by SPARQL-Star Virtual ID resolution: a Virtual ID is the FNV-1a hash of an embedded
    /// triple, stored in the lexicon with tag `LEX_TAG_EMBEDDED` instead of `LEX_TAG_STRING`.
    pub fn lookup_embedded_triple(&self, hash: u64) -> Option<[u64; 3]> {
        if self.format_version == LEX_VERSION_PAGED {
            return None;
        }
        let mut lo = 0usize;
        let mut hi = self.entry_count;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let off = HEADER_SIZE + mid * INDEX_ENTRY_SIZE;
            let entry_hash = u64::from_le_bytes(self.data[off..off + 8].try_into().ok()?);
            match entry_hash.cmp(&hash) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => {
                    let str_off =
                        u64::from_le_bytes(self.data[off + 8..off + 16].try_into().ok()?) as usize;
                    return Self::read_embedded_triple_at(self.data, self.strings_offset, str_off)
                        .copied();
                }
            }
        }
        None
    }

    /// Binary search for `hash`; returns the authoritative Webizen identity string.
    pub fn lookup_webizen_identity(&self, hash: u64) -> Option<&'a str> {
        if self.format_version == LEX_VERSION_PAGED {
            return None;
        }
        let mut lo = 0usize;
        let mut hi = self.entry_count;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let off = HEADER_SIZE + mid * INDEX_ENTRY_SIZE;
            let entry_hash = u64::from_le_bytes(self.data[off..off + 8].try_into().ok()?);
            match entry_hash.cmp(&hash) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => {
                    let str_off =
                        u64::from_le_bytes(self.data[off + 8..off + 16].try_into().ok()?) as usize;
                    return Self::read_webizen_at(self.data, self.strings_offset, str_off);
                }
            }
        }
        None
    }

    fn read_webizen_at(data: &[u8], blob_base: usize, rel_off: usize) -> Option<&str> {
        let start = blob_base.checked_add(rel_off)?;
        if start.checked_add(3)? > data.len() || data[start] != LEX_TAG_WEBIZEN {
            return None;
        }
        let len = u16::from_le_bytes(data[start + 1..start + 3].try_into().ok()?) as usize;
        let text_start = start + 3;
        let text_end = text_start.checked_add(len)?;
        if text_end > data.len() {
            return None;
        }
        std::str::from_utf8(&data[text_start..text_end]).ok()
    }

    /// Reads a 24-byte embedded triple [u64; 3] at the given offset.
    ///
    /// Format: [TAG_EMBEDDED (1 byte)] + [24-byte triple]
    fn read_embedded_triple_at(data: &[u8], blob_base: usize, rel_off: usize) -> Option<&[u64; 3]> {
        let start = blob_base.saturating_add(rel_off);
        if start + 1 + 24 > data.len() {
            return None;
        }
        // Check type tag
        if data[start] != LEX_TAG_EMBEDDED {
            return None;
        }
        // Skip type tag and read 24-byte triple
        let triple_start = start + 1;
        let bytes = &data[triple_start..triple_start + 24];
        let ptr = bytes.as_ptr() as *const [u64; 3];
        unsafe { Some(&*ptr) }
    }
}

/// Memory-mapped `.q42.lex` file handle (native targets).
#[cfg(not(target_arch = "wasm32"))]
pub struct Q42LexFile {
    mmap: Mmap,
}

#[cfg(not(target_arch = "wasm32"))]
impl Q42LexFile {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        Q42LexMmap::from_bytes(&mmap)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{e:?}")))?;
        Ok(Self { mmap })
    }

    #[inline]
    pub fn view(&self) -> Q42LexMmap<'_> {
        Q42LexMmap::from_bytes(&self.mmap).expect("validated at open")
    }
}

impl Q42Lexicon {
    /// Load lexicon embedded in a unified v2 `.q42` volume or a legacy `.q42.lex` sidecar.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_for_q42(q42_path: &Path) -> std::io::Result<Self> {
        if crate::q42_volume::is_unified_volume(q42_path)? {
            let vol = crate::q42_volume::Q42Volume::open(q42_path)?;
            if vol.volume_manifest()?.is_some() {
                let set = crate::q42_volume::Q42VolumeSet::open_root(q42_path)?;
                let mut lexicon = Self::load_from_lex_bytes(set.root().lex_bytes())?;
                for shard in set.lexicon_segments() {
                    let entries = Self::load_from_lex_bytes(shard.lex_bytes())?;
                    lexicon.entries.extend(entries.entries);
                }
                return Ok(lexicon);
            }
            return Self::load_from_lex_bytes(vol.lex_bytes());
        }
        let sidecar = q42_path.with_extension("q42.lex");
        if sidecar.is_file() {
            return Self::load(&sidecar);
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "no lexicon in {} or sidecar {}",
                q42_path.display(),
                sidecar.display()
            ),
        ))
    }

    /// Build an in-memory lexicon from a Q42LEX byte slice (embedded or sidecar).
    pub fn load_from_lex_bytes(data: &[u8]) -> std::io::Result<Self> {
        let view = Q42LexMmap::from_bytes(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{e:?}")))?;
        let mut entries = HashMap::with_capacity(view.entry_count());
        for i in 0..view.entry_count() {
            let Some(hash) = view.hash_at(i) else {
                break;
            };
            if let Some(text) = view.lookup_hash(hash) {
                entries.insert(hash, text.to_string());
            }
        }
        Ok(Self { entries })
    }

    pub fn load(path: &Path) -> std::io::Result<Self> {
        let mut file = File::open(path)?;
        let mut header = [0u8; 32];
        file.read_exact(&mut header)?;
        if header[0..8] != *MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid Q42LEX magic",
            ));
        }
        let entry_count = u64::from_le_bytes(header[8..16].try_into().unwrap()) as usize;
        let strings_offset = u64::from_le_bytes(header[16..24].try_into().unwrap()) as usize;

        let mut index_buf = vec![0u8; entry_count * 16];
        file.read_exact(&mut index_buf)?;

        let mut blob = Vec::new();
        file.seek(std::io::SeekFrom::Start(strings_offset as u64))?;
        file.read_to_end(&mut blob)?;

        let mut entries = HashMap::with_capacity(entry_count);
        for i in 0..entry_count {
            let off = i * 16;
            let hash = u64::from_le_bytes(index_buf[off..off + 8].try_into().unwrap());
            let str_off =
                u64::from_le_bytes(index_buf[off + 8..off + 16].try_into().unwrap()) as usize;
            if str_off + 2 > blob.len() {
                continue;
            }
            let len = u16::from_le_bytes(blob[str_off..str_off + 2].try_into().unwrap()) as usize;
            let start = str_off + 2;
            let end = start.saturating_add(len).min(blob.len());
            if let Ok(text) = std::str::from_utf8(&blob[start..end]) {
                entries.insert(hash, text.to_string());
            }
        }

        Ok(Self { entries })
    }

    pub fn lookup(&self, hash: u64) -> Option<&str> {
        self.entries.get(&hash).map(|s| s.as_str())
    }

    /// Find first lexicon entry whose lowercase text equals `needle`.
    pub fn find_literal(&self, needle: &str) -> Option<u64> {
        let needle = needle.to_lowercase();
        self.entries
            .iter()
            .find(|(_, v)| v.to_lowercase() == needle)
            .map(|(h, _)| *h)
    }

    /// Entries whose text contains `sub` (case-insensitive), capped.
    pub fn search_contains(&self, sub: &str, limit: usize) -> Vec<(u64, String)> {
        let sub = sub.to_lowercase();
        let mut out = Vec::new();
        for (h, v) in &self.entries {
            if v.to_lowercase().contains(&sub) {
                out.push((*h, v.clone()));
                if out.len() >= limit {
                    break;
                }
            }
        }
        out
    }
}

use std::io::Seek;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_lex_bytes(entries: &[(u64, &str)]) -> Vec<u8> {
        let mut sorted: Vec<(u64, &str)> = entries.to_vec();
        sorted.sort_unstable_by_key(|(h, _)| *h);
        let entry_count = sorted.len() as u64;
        let strings_offset = 32 + entry_count * 16;
        let mut blob = Vec::new();
        let mut index = Vec::new();
        for (hash, text) in &sorted {
            let str_off = blob.len() as u64;
            // Write type tag
            blob.push(LEX_TAG_STRING);
            let b = text.as_bytes();
            let len = b.len().min(65535) as u16;
            blob.extend_from_slice(&len.to_le_bytes());
            blob.extend_from_slice(&b[..len as usize]);
            index.extend_from_slice(&hash.to_le_bytes());
            index.extend_from_slice(&str_off.to_le_bytes());
        }
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&entry_count.to_le_bytes());
        out.extend_from_slice(&strings_offset.to_le_bytes());
        out.extend_from_slice(&1u64.to_le_bytes());
        out.extend_from_slice(&index);
        out.extend_from_slice(&blob);
        out
    }

    #[test]
    fn mmap_lex_binary_search() {
        let h1 = crate::q_hash("Patient");
        let h2 = crate::q_hash("fever");
        let bytes = write_lex_bytes(&[(h1, "Patient"), (h2, "fever")]);
        let lex = Q42LexMmap::from_bytes(&bytes).unwrap();
        assert_eq!(lex.lookup_hash(h1), Some("Patient"));
        assert_eq!(lex.lookup_hash(h2), Some("fever"));
        assert_eq!(lex.lookup_hash(0xDEAD), None);
    }

    /// The write side ([`serialize_string_lexicon`]) round-trips through the read side for the full
    /// Unicode range — multilingual literals (non-Latin scripts, combining marks, emoji) come back
    /// byte-identical. This is the property that makes lossless ingest actually lossless for the
    /// world's languages, not just ASCII.
    #[test]
    fn serialize_lexicon_round_trips_unicode() {
        let samples = [
            "carefully",                        // ASCII (WordNet eng gloss word)
            "välinpitämättömästi",              // Finnish (WordNet fin)
            "อย่างสะเพร่า",                       // Thai
            "不注意に",                         // Japanese
            "بلا مبالاة",                         // Arabic (RTL)
            "невнимательно",                    // Russian (Cyrillic)
            "a definition — with em-dash & 😀", // punctuation + emoji (4-byte codepoint)
        ];
        let mut map = HashMap::new();
        for s in samples {
            map.insert(crate::q_hash(s), s.to_string());
        }
        let bytes = serialize_string_lexicon(&map).unwrap();
        let lex = Q42LexMmap::from_bytes(&bytes).unwrap();
        assert_eq!(lex.entry_count(), samples.len());
        for s in samples {
            assert_eq!(
                lex.lookup_hash(crate::q_hash(s)),
                Some(s),
                "multilingual lexeme must round-trip byte-identical: {s:?}"
            );
        }
    }

    /// A literal longer than the 16-bit length field is truncated at a char boundary, never
    /// mid-codepoint — so the stored bytes are always valid UTF-8 and the reader returns `Some`.
    #[test]
    fn serialize_lexicon_rejects_overlong_term() {
        // 22-000 three-byte codepoints ≈ 66 000 bytes > u16::MAX (65 535); the cut lands between
        // codepoints, so from_utf8 on read still succeeds.
        let long: String = "あ".repeat(22_000);
        let h = crate::q_hash(&long);
        let mut map = HashMap::new();
        map.insert(h, long);
        assert_eq!(serialize_string_lexicon(&map), Err(LexError::TermTooLong));
    }

    #[test]
    fn paged_lexicon_binary_search_crosses_page_boundaries() {
        let mut map = HashMap::new();
        for value in 0u64..9 {
            map.insert(value * 10 + 3, format!("urn:q42:paged:{value}"));
        }
        let bytes = serialize_paged_string_lexicon(&map, 2).unwrap();
        assert_eq!(
            u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
            LEX_VERSION_PAGED
        );
        let view = Q42LexMmap::from_bytes(&bytes).unwrap();
        assert_eq!(view.entry_count(), 9);
        assert_eq!(view.lookup_hash(3), Some("urn:q42:paged:0"));
        assert_eq!(view.lookup_hash(83), Some("urn:q42:paged:8"));
        assert_eq!(view.string_at(4), Some("urn:q42:paged:4"));
        assert_eq!(view.lookup_hash(4), None);
        let cold = Q42Lexicon::load_from_lex_bytes(&bytes).unwrap();
        assert_eq!(cold.lookup(43), Some("urn:q42:paged:4"));
    }

    #[test]
    fn mmap_lex_file_roundtrip() {
        let h = crate::q_hash("Entity");
        let bytes = write_lex_bytes(&[(h, "Entity")]);
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(&bytes).unwrap();
        let file = Q42LexFile::open(tmp.path()).unwrap();
        assert_eq!(file.view().lookup_hash(h), Some("Entity"));
    }
}
