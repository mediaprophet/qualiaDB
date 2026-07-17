//! Content-addressed media store (Phase 2 / V2) — pure Rust, no Python.
//!
//! Layout under `root`:
//! ```text
//! media/
//!   by-hash/
//!     ab/cd/<hex>.bin     # raw bytes
//!   index/
//!     <hex>.json          # MediaRecord
//! ```

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::semantic::media_digest;

/// Retention / sensitivity class for media (mirrors graph sensitivity language).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RetentionClass {
    Public = 0,
    Restricted = 1,
    Classified = 2,
}

impl RetentionClass {
    pub fn as_str(self) -> &'static str {
        match self {
            RetentionClass::Public => "public",
            RetentionClass::Restricted => "restricted",
            RetentionClass::Classified => "classified",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "public" => Some(Self::Public),
            "restricted" => Some(Self::Restricted),
            "classified" => Some(Self::Classified),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaRecord {
    pub digest_hex: String,
    pub byte_len: u64,
    pub mime: String,
    pub width: u32,
    pub height: u32,
    pub retention: RetentionClass,
    pub imported_unix: u64,
    /// FNV digest hash (u64) for quin linking.
    pub digest_u64: u64,
}

pub struct MediaStore {
    root: PathBuf,
}

impl MediaStore {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, String> {
        let root = root.into();
        fs::create_dir_all(root.join("media/by-hash")).map_err(|e| e.to_string())?;
        fs::create_dir_all(root.join("media/index")).map_err(|e| e.to_string())?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn blob_path(&self, digest_hex: &str) -> PathBuf {
        let a = digest_hex.get(0..2).unwrap_or("00");
        let b = digest_hex.get(2..4).unwrap_or("00");
        self.root
            .join("media/by-hash")
            .join(a)
            .join(b)
            .join(format!("{digest_hex}.bin"))
    }

    fn index_path(&self, digest_hex: &str) -> PathBuf {
        self.root
            .join("media/index")
            .join(format!("{digest_hex}.json"))
    }

    /// Import raw bytes. Same content → same digest (dedupe). Partial writes never leave an index.
    pub fn import_bytes(
        &self,
        bytes: &[u8],
        mime: &str,
        width: u32,
        height: u32,
        retention: RetentionClass,
        now_unix: u64,
    ) -> Result<MediaRecord, String> {
        if bytes.is_empty() {
            return Err("empty media".into());
        }
        let d = media_digest(bytes);
        let digest_hex = format!("{:016x}", d.hash);
        let blob = self.blob_path(&digest_hex);
        let idx = self.index_path(&digest_hex);

        if blob.is_file() && idx.is_file() {
            return self.read_record(&digest_hex);
        }

        if let Some(parent) = blob.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let tmp = blob.with_extension("bin.tmp");
        {
            let mut f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
            f.write_all(bytes).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
        }
        fs::rename(&tmp, &blob).map_err(|e| e.to_string())?;

        let rec = MediaRecord {
            digest_hex: digest_hex.clone(),
            byte_len: bytes.len() as u64,
            mime: mime.into(),
            width,
            height,
            retention,
            imported_unix: now_unix,
            digest_u64: d.hash,
        };
        let json = format!(
            "{{\n  \"digest_hex\": \"{}\",\n  \"byte_len\": {},\n  \"mime\": \"{}\",\n  \"width\": {},\n  \"height\": {},\n  \"retention\": \"{}\",\n  \"imported_unix\": {},\n  \"digest_u64\": {}\n}}\n",
            rec.digest_hex,
            rec.byte_len,
            escape_json(&rec.mime),
            rec.width,
            rec.height,
            rec.retention.as_str(),
            rec.imported_unix,
            rec.digest_u64
        );
        let idx_tmp = idx.with_extension("json.tmp");
        fs::write(&idx_tmp, json.as_bytes()).map_err(|e| e.to_string())?;
        fs::rename(&idx_tmp, &idx).map_err(|e| e.to_string())?;
        Ok(rec)
    }

    pub fn read_record(&self, digest_hex: &str) -> Result<MediaRecord, String> {
        let s = fs::read_to_string(self.index_path(digest_hex)).map_err(|e| e.to_string())?;
        parse_record_json(&s)
    }

    pub fn read_bytes(&self, digest_hex: &str) -> Result<Vec<u8>, String> {
        fs::read(self.blob_path(digest_hex)).map_err(|e| e.to_string())
    }

    pub fn exists(&self, digest_hex: &str) -> bool {
        self.blob_path(digest_hex).is_file() && self.index_path(digest_hex).is_file()
    }
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn parse_record_json(s: &str) -> Result<MediaRecord, String> {
    // Minimal hand parser for our fixed schema (no serde dep on vision crate).
    fn field<'a>(s: &'a str, key: &str) -> Result<&'a str, String> {
        let pat = format!("\"{key}\"");
        let i = s
            .find(&pat)
            .ok_or_else(|| format!("missing {key}"))?;
        let rest = &s[i + pat.len()..];
        let rest = rest.trim_start_matches(|c: char| c == ' ' || c == ':' || c == '\t');
        if rest.starts_with('"') {
            let rest = &rest[1..];
            let end = rest.find('"').ok_or_else(|| format!("bad string {key}"))?;
            Ok(&rest[..end])
        } else {
            let end = rest
                .find(|c: char| c == ',' || c == '}' || c == '\n')
                .unwrap_or(rest.len());
            Ok(rest[..end].trim())
        }
    }
    let digest_hex = field(s, "digest_hex")?.to_string();
    let byte_len: u64 = field(s, "byte_len")?
        .parse()
        .map_err(|e| format!("byte_len: {e}"))?;
    let mime = field(s, "mime")?.to_string();
    let width: u32 = field(s, "width")?
        .parse()
        .map_err(|e| format!("width: {e}"))?;
    let height: u32 = field(s, "height")?
        .parse()
        .map_err(|e| format!("height: {e}"))?;
    let retention = RetentionClass::parse(field(s, "retention")?)
        .ok_or_else(|| "bad retention".to_string())?;
    let imported_unix: u64 = field(s, "imported_unix")?
        .parse()
        .map_err(|e| format!("imported_unix: {e}"))?;
    let digest_u64: u64 = field(s, "digest_u64")?
        .parse()
        .map_err(|e| format!("digest_u64: {e}"))?;
    Ok(MediaRecord {
        digest_hex,
        byte_len,
        mime,
        width,
        height,
        retention,
        imported_unix,
        digest_u64,
    })
}

/// Map store errors into VisionError when used at ABI edge.
pub fn map_store_err(e: String) -> VisionError {
    let _ = e;
    VisionError::BackendUnavailable
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn import_dedupes() {
        let dir = std::env::temp_dir().join(format!(
            "qv-media-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        let store = MediaStore::open(&dir).unwrap();
        let bytes = b"\x00\xff\x00hello-vision-fixture";
        let r1 = store
            .import_bytes(bytes, "application/octet-stream", 1, 1, RetentionClass::Public, 1)
            .unwrap();
        let r2 = store
            .import_bytes(bytes, "application/octet-stream", 1, 1, RetentionClass::Public, 2)
            .unwrap();
        assert_eq!(r1.digest_hex, r2.digest_hex);
        assert!(store.exists(&r1.digest_hex));
        assert_eq!(store.read_bytes(&r1.digest_hex).unwrap(), bytes);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn digest_matches_semantic() {
        let b = [1u8, 2, 3, 4];
        let d = media_digest(&b);
        assert_eq!(d.byte_len, 4);
        assert_ne!(d.hash, 0);
    }
}
