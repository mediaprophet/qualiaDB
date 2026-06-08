//! Read `.q42.lex` reverse-lexicon sidecars (Q42LEX format from qualia-cli ingest).

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use memmap2::Mmap;

const MAGIC: &[u8; 8] = b"Q42LEX\0\0";
const HEADER_SIZE: usize = 32;
const INDEX_ENTRY_SIZE: usize = 16;

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
            let entry_hash = u64::from_le_bytes(
                self.data[off..off + 8]
                    .try_into()
                    .ok()?,
            );
            match entry_hash.cmp(&hash) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => {
                    let str_off = u64::from_le_bytes(
                        self.data[off + 8..off + 16]
                            .try_into()
                            .ok()?,
                    ) as usize;
                    return Self::read_string_at(self.data, self.strings_offset, str_off);
                }
            }
        }
        None
    }

    fn read_string_at(data: &[u8], blob_base: usize, rel_off: usize) -> Option<&str> {
        let start = blob_base.saturating_add(rel_off);
        if start + 2 > data.len() {
            return None;
        }
        let len = u16::from_le_bytes(data[start..start + 2].try_into().ok()?) as usize;
        let text_start = start + 2;
        let text_end = text_start.saturating_add(len).min(data.len());
        std::str::from_utf8(&data[text_start..text_end]).ok()
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
        Q42LexMmap::from_bytes(&mmap).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{e:?}"))
        })?;
        Ok(Self { mmap })
    }

    #[inline]
    pub fn view(&self) -> Q42LexMmap<'_> {
        Q42LexMmap::from_bytes(&self.mmap).expect("validated at open")
    }
}

impl Q42Lexicon {
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
            let str_off = u64::from_le_bytes(index_buf[off + 8..off + 16].try_into().unwrap()) as usize;
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
