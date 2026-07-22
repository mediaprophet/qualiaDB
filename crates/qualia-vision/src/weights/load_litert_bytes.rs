//! Cold-path Google LiteRT (TFLite / FlatBuffers) model file validation.
//!
//! LiteRT / TFLite models use the FlatBuffers binary format with the canonical
//! 4-byte file identifier `TFL3` at byte offset 4..8.

use std::fs;
use std::path::Path;

/// Canonical FlatBuffers 4-byte identifier for Google LiteRT / TFLite models.
pub const LITER_TFLITE_MAGIC: [u8; 4] = *b"TFL3";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteRtFileMeta {
    pub bytes: usize,
    pub looks_like_litert: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteRtLoadError {
    Io,
    TooSmall,
    NotLiteRtLike,
}

const MIN_LITERT_BYTES: usize = 64;

/// Load entire LiteRT / TFLite file into memory (cold path).
pub fn load_litert_file(path: &Path) -> Result<(Vec<u8>, LiteRtFileMeta), LiteRtLoadError> {
    let data = fs::read(path).map_err(|_| LiteRtLoadError::Io)?;
    let meta = validate_litert_bytes(&data)?;
    Ok((data, meta))
}

/// Validate Google LiteRT / TFLite FlatBuffers bytes without reading disk.
pub fn validate_litert_bytes(data: &[u8]) -> Result<LiteRtFileMeta, LiteRtLoadError> {
    if data.len() < MIN_LITERT_BYTES {
        return Err(LiteRtLoadError::TooSmall);
    }
    let looks = looks_like_litert(data);
    if !looks {
        return Err(LiteRtLoadError::NotLiteRtLike);
    }
    Ok(LiteRtFileMeta {
        bytes: data.len(),
        looks_like_litert: true,
    })
}

fn looks_like_litert(data: &[u8]) -> bool {
    // Canonical FlatBuffer ID at offset 4..8 is "TFL3"
    if data.len() >= 8 && &data[4..8] == &LITER_TFLITE_MAGIC {
        return true;
    }
    // Backup search in first 256 bytes for TFL3, TFLITE, or LiteRT
    let head = &data[..data.len().min(256)];
    find_subslice(head, b"TFL3") || find_subslice(head, b"TFLITE") || find_subslice(head, b"LiteRT")
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_litert_validation() {
        let mut fake_tflite = vec![0u8; 128];
        fake_tflite[4..8].copy_from_slice(b"TFL3");

        let meta = validate_litert_bytes(&fake_tflite).unwrap();
        assert!(meta.looks_like_litert);
        assert_eq!(meta.bytes, 128);

        let small = vec![0u8; 10];
        assert_eq!(validate_litert_bytes(&small), Err(LiteRtLoadError::TooSmall));

        let invalid = vec![0u8; 128];
        assert_eq!(validate_litert_bytes(&invalid), Err(LiteRtLoadError::NotLiteRtLike));
    }
}
