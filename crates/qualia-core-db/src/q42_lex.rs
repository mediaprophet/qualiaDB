//! Read `.q42.lex` reverse-lexicon sidecars (Q42LEX format from qualia-cli ingest).

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use memmap2::Mmap;

pub const LEX_MAGIC: [u8; 8] = *b"Q42LEX\0\0";
const MAGIC: &[u8; 8] = &LEX_MAGIC;
const HEADER_SIZE: usize = 32;
const INDEX_ENTRY_SIZE: usize = 16;

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
pub fn serialize_string_lexicon(entries: &HashMap<u64, String>) -> Vec<u8> {
    let mut sorted: Vec<(&u64, &String)> = entries.iter().collect();
    sorted.sort_unstable_by_key(|(h, _)| **h);
    let entry_count = sorted.len() as u64;
    let strings_offset = HEADER_SIZE as u64 + entry_count * INDEX_ENTRY_SIZE as u64;

    let mut index: Vec<u8> = Vec::with_capacity(sorted.len() * INDEX_ENTRY_SIZE);
    let mut blob: Vec<u8> = Vec::new();
    for (hash, text) in &sorted {
        let str_off = blob.len() as u64;
        let s = utf8_prefix(text, u16::MAX as usize);
        blob.push(LEX_TAG_STRING);
        blob.extend_from_slice(&(s.len() as u16).to_le_bytes());
        blob.extend_from_slice(s.as_bytes());
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
    out
}

/// Largest UTF-8 prefix of `s` that fits in `max_bytes`, never splitting a codepoint.
fn utf8_prefix(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

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
}

/// Zero-allocation view over a memory-mapped `.q42.lex` slice (sorted hash index).
#[derive(Debug, Clone, Copy)]
pub struct Q42LexMmap<'a> {
    data: &'a [u8],
    entry_count: usize,
    strings_offset: usize,
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
        let entry_count = u64::from_le_bytes(data[8..16].try_into().unwrap()) as usize;
        let strings_offset = u64::from_le_bytes(data[16..24].try_into().unwrap()) as usize;
        let index_end = HEADER_SIZE.saturating_add(entry_count.saturating_mul(INDEX_ENTRY_SIZE));
        if index_end > data.len() || strings_offset > data.len() {
            return Err(LexError::Truncated);
        }
        Ok(Self {
            data,
            entry_count,
            strings_offset,
        })
    }

    #[inline]
    pub fn entry_count(&self) -> usize {
        self.entry_count
    }

    /// Binary search for `hash` in the sorted index; returns the UTF-8 lexeme slice.
    pub fn lookup_hash(&self, hash: u64) -> Option<&'a str> {
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
        let off = HEADER_SIZE + i * INDEX_ENTRY_SIZE;
        let str_off = u64::from_le_bytes(self.data[off + 8..off + 16].try_into().ok()?) as usize;
        Self::read_string_at(self.data, self.strings_offset, str_off)
    }

    fn read_string_at(data: &[u8], blob_base: usize, rel_off: usize) -> Option<&str> {
        let start = blob_base.saturating_add(rel_off);
        if start + 3 > data.len() {
            return None;
        }
        // Check type tag
        if data[start] != LEX_TAG_STRING {
            return None;
        }
        let len = u16::from_le_bytes(data[start + 1..start + 3].try_into().ok()?) as usize;
        let text_start = start + 3;
        let text_end = text_start.saturating_add(len).min(data.len());
        std::str::from_utf8(&data[text_start..text_end]).ok()
    }

    /// Binary search for `hash` in the sorted index; returns the embedded triple [subject, predicate, object].
    ///
    /// Used by SPARQL-Star Virtual ID resolution: a Virtual ID is the FNV-1a hash of an embedded
    /// triple, stored in the lexicon with tag `LEX_TAG_EMBEDDED` instead of `LEX_TAG_STRING`.
    pub fn lookup_embedded_triple(&self, hash: u64) -> Option<[u64; 3]> {
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
        let start = blob_base.saturating_add(rel_off);
        if start + 3 > data.len() || data[start] != LEX_TAG_WEBIZEN {
            return None;
        }
        let len = u16::from_le_bytes(data[start + 1..start + 3].try_into().ok()?) as usize;
        let text_start = start + 3;
        let text_end = text_start.saturating_add(len).min(data.len());
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
            let off = HEADER_SIZE + i * INDEX_ENTRY_SIZE;
            if off + INDEX_ENTRY_SIZE > data.len() {
                break;
            }
            let hash = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
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
        let bytes = serialize_string_lexicon(&map);
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
    fn serialize_lexicon_truncates_on_char_boundary() {
        // 22-000 three-byte codepoints ≈ 66 000 bytes > u16::MAX (65 535); the cut lands between
        // codepoints, so from_utf8 on read still succeeds.
        let long: String = "あ".repeat(22_000);
        let h = crate::q_hash(&long);
        let mut map = HashMap::new();
        map.insert(h, long);
        let bytes = serialize_string_lexicon(&map);
        let lex = Q42LexMmap::from_bytes(&bytes).unwrap();
        let got = lex
            .lookup_hash(h)
            .expect("truncated string still valid UTF-8");
        assert!(got.len() <= u16::MAX as usize);
        assert!(got.chars().all(|c| c == 'あ'));
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
