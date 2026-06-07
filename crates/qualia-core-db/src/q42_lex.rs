//! Read `.q42.lex` reverse-lexicon sidecars (Q42LEX format from qualia-cli ingest).

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

const MAGIC: &[u8; 8] = b"Q42LEX\0\0";

/// In-memory hash → UTF-8 string map from a `.q42.lex` file.
#[derive(Debug, Default)]
pub struct Q42Lexicon {
    pub entries: HashMap<u64, String>,
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
