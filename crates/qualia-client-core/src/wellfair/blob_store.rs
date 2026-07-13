//! Content-addressed blob store for WellFair payloads (master plan §5).
//!
//! Large or structured payloads that don't belong inline on the journal — credential claim
//! sets, clinical attachment bytes, letter scans — are stored here as content-addressed blobs.
//! Their identifier is the SHA-256 (hex) of the bytes, which is exactly the `blob_hash` the
//! record envelope already carries, so a journal row's `blob_hash` is a direct handle to its
//! blob. Writes are idempotent (same bytes → same file) and reads verify integrity.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

pub const BLOB_DIR: &str = "wellfair/blobs";

/// Content-addressed store rooted under `{storage_root}/wellfair/blobs`.
pub struct BlobStore {
    dir: PathBuf,
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// A content hash is a 64-char lowercase hex string; reject anything else so a hostile or
/// malformed handle can never escape the blob directory (path-traversal defense).
fn is_valid_hash(hash_hex: &str) -> bool {
    hash_hex.len() == 64 && hash_hex.bytes().all(|b| b.is_ascii_hexdigit())
}

impl BlobStore {
    pub fn open(storage_root: impl AsRef<Path>) -> std::io::Result<Self> {
        let dir = storage_root.as_ref().join(BLOB_DIR);
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    fn path_for(&self, hash_hex: &str) -> Option<PathBuf> {
        if is_valid_hash(hash_hex) {
            Some(self.dir.join(hash_hex))
        } else {
            None
        }
    }

    /// Store bytes, returning their content hash. Idempotent: re-storing identical bytes is a
    /// no-op that returns the same hash. Written via a temp file + rename so a reader never sees
    /// a partial blob.
    pub fn put(&self, bytes: &[u8]) -> std::io::Result<String> {
        let hash = sha256_hex(bytes);
        let path = self.dir.join(&hash);
        if path.exists() {
            return Ok(hash);
        }
        let tmp = self.dir.join(format!("{hash}.tmp"));
        {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&tmp)?;
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        fs::rename(&tmp, &path)?;
        Ok(hash)
    }

    /// Read a blob by content hash, verifying integrity. Returns `Ok(None)` if absent, and an
    /// error if the stored bytes no longer hash to the requested handle (corruption/tamper).
    pub fn get(&self, hash_hex: &str) -> std::io::Result<Option<Vec<u8>>> {
        let Some(path) = self.path_for(hash_hex) else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid blob hash",
            ));
        };
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&path)?;
        if sha256_hex(&bytes) != hash_hex {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "blob content hash mismatch (corrupt or tampered)",
            ));
        }
        Ok(Some(bytes))
    }

    pub fn exists(&self, hash_hex: &str) -> bool {
        self.path_for(hash_hex)
            .map(|p| p.exists())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_returns_content_hash_and_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path()).unwrap();
        let payload = b"{\"claims\":[[\"postcode\",\"3000\"]]}";
        let hash = store.put(payload).unwrap();
        assert_eq!(hash.len(), 64);
        assert!(store.exists(&hash));
        assert_eq!(store.get(&hash).unwrap().unwrap(), payload);
    }

    #[test]
    fn put_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path()).unwrap();
        let h1 = store.put(b"same bytes").unwrap();
        let h2 = store.put(b"same bytes").unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn get_missing_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path()).unwrap();
        let absent = sha256_hex(b"never stored");
        assert!(store.get(&absent).unwrap().is_none());
    }

    #[test]
    fn invalid_hash_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path()).unwrap();
        // Path-traversal attempt and wrong-length handles must not resolve.
        assert!(store.get("../secret").is_err());
        assert!(store.get("deadbeef").is_err());
        assert!(!store.exists("../secret"));
    }

    #[test]
    fn tampered_blob_fails_integrity() {
        let dir = tempfile::tempdir().unwrap();
        let store = BlobStore::open(dir.path()).unwrap();
        let hash = store.put(b"original").unwrap();
        // Overwrite the stored file with different bytes under the same name.
        fs::write(dir.path().join(BLOB_DIR).join(&hash), b"tampered").unwrap();
        assert!(store.get(&hash).is_err());
    }

    #[test]
    fn survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let hash = {
            let store = BlobStore::open(dir.path()).unwrap();
            store.put(b"persisted payload").unwrap()
        };
        let reopened = BlobStore::open(dir.path()).unwrap();
        assert_eq!(reopened.get(&hash).unwrap().unwrap(), b"persisted payload");
    }
}
