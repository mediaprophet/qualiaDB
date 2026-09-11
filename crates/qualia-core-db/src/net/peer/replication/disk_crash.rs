//! Process-local tempfile crash/recovery (E06.6).
//!
//! Commits identity → effect bytes → durable receipt marker as separate files.
//! A receipt file without complete effect bytes is never a durable receipt.
//! [`os_process_kill_qualified`] stays false: this is not an OS kill of CORE-03 WAL.

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::crypto::network::digest::sha384;
use crate::net::peer::replication::receipts::ReceiptClass;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Max effect payload retained on this path.
pub const MAX_EFFECT_BYTES: usize = 4096;

/// This tempfile adapter is injected. Not an OS process-kill qualifier.
#[inline]
pub const fn tempfile_crash_injected() -> bool {
    true
}

/// OS process-kill / CORE-03 WAL remaining unproven.
#[inline]
pub const fn os_process_kill_qualified() -> bool {
    false
}

/// Bounded on-disk pairing of identity, effect, and receipt.
pub struct FilePairStore {
    dir: PathBuf,
}

impl FilePairStore {
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, QdnfError> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir).map_err(|_| QdnfError::Incomplete)?;
        Ok(Self {
            dir: dir.to_path_buf(),
        })
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// Write identity digest first.
    pub fn write_identity(&self, id: StrongDigest) -> Result<(), QdnfError> {
        if id == StrongDigest::ZERO {
            return Err(QdnfError::Malformed);
        }
        write_exact(&self.path("identity"), id.as_bytes())
    }

    /// Write exact effect bytes. Empty or oversize is Malformed/Capacity.
    pub fn write_effect(&self, bytes: &[u8]) -> Result<StrongDigest, QdnfError> {
        if bytes.is_empty() {
            return Err(QdnfError::Malformed);
        }
        if bytes.len() > MAX_EFFECT_BYTES {
            return Err(QdnfError::Capacity);
        }
        write_exact(&self.path("effect"), bytes)?;
        Ok(sha384(bytes))
    }

    /// Durable receipt marker. Requires a complete effect file whose digest matches.
    pub fn write_receipt(&self, effect_digest: StrongDigest) -> Result<(), QdnfError> {
        match self.recover()? {
            RecoveredPair {
                receipt_class: ReceiptClass::Effect,
                effect_digest: Some(got),
                ..
            } if got == effect_digest => write_exact(&self.path("receipt"), effect_digest.as_bytes()),
            _ => Err(QdnfError::Incomplete),
        }
    }

    /// Reconstruct pairing. Truncation/corruption is Conflict or Incomplete.
    pub fn recover(&self) -> Result<RecoveredPair, QdnfError> {
        let identity = match read_digest(&self.path("identity")) {
            Ok(id) => id,
            Err(QdnfError::Incomplete) => {
                return Ok(RecoveredPair {
                    identity: None,
                    effect_digest: None,
                    receipt_class: ReceiptClass::Identity,
                });
            }
            Err(e) => return Err(e),
        };
        let effect = match read_bounded(&self.path("effect"), MAX_EFFECT_BYTES) {
            Ok(bytes) => Some(sha384(&bytes)),
            Err(QdnfError::Incomplete) => None,
            Err(e) => return Err(e),
        };
        let receipt = match read_digest(&self.path("receipt")) {
            Ok(d) => Some(d),
            Err(QdnfError::Incomplete) => None,
            Err(e) => return Err(e),
        };
        let receipt_class = match (effect, receipt) {
            (Some(ed), Some(rd)) if ed == rd => ReceiptClass::DurableReceipt,
            (Some(_), Some(_)) => return Err(QdnfError::Conflict),
            (None, Some(_)) => return Err(QdnfError::Incomplete),
            (Some(_), None) => ReceiptClass::Effect,
            (None, None) => ReceiptClass::Identity,
        };
        Ok(RecoveredPair {
            identity: Some(identity),
            effect_digest: effect,
            receipt_class,
        })
    }

    /// Truncate the effect file to simulate a torn write.
    pub fn truncate_effect(&self, len: u64) -> Result<(), QdnfError> {
        let f = OpenOptions::new()
            .write(true)
            .open(self.path("effect"))
            .map_err(|_| QdnfError::Incomplete)?;
        f.set_len(len).map_err(|_| QdnfError::Incomplete)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecoveredPair {
    pub identity: Option<StrongDigest>,
    pub effect_digest: Option<StrongDigest>,
    pub receipt_class: ReceiptClass,
}

fn write_exact(path: &Path, bytes: &[u8]) -> Result<(), QdnfError> {
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|_| QdnfError::Incomplete)?;
    f.write_all(bytes).map_err(|_| QdnfError::Incomplete)?;
    f.sync_all().map_err(|_| QdnfError::Incomplete)
}

fn read_bounded(path: &Path, max: usize) -> Result<Vec<u8>, QdnfError> {
    let meta = fs::metadata(path).map_err(|_| QdnfError::Incomplete)?;
    let len = usize::try_from(meta.len()).map_err(|_| QdnfError::Range)?;
    if len == 0 {
        return Err(QdnfError::Incomplete);
    }
    if len > max {
        return Err(QdnfError::Conflict);
    }
    let mut buf = vec![0u8; len];
    let mut f = fs::File::open(path).map_err(|_| QdnfError::Incomplete)?;
    f.read_exact(&mut buf).map_err(|_| QdnfError::Incomplete)?;
    Ok(buf)
}

fn read_digest(path: &Path) -> Result<StrongDigest, QdnfError> {
    let bytes = read_bounded(path, 48)?;
    if bytes.len() != 48 {
        return Err(QdnfError::Conflict);
    }
    let mut d = StrongDigest::ZERO;
    d.0.copy_from_slice(&bytes);
    if d == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn id() -> StrongDigest {
        sha384(b"op-identity")
    }

    #[test]
    fn tempfile_torn_write_recovers_consistent_pair() {
        let dir = TempDir::new().unwrap();
        let store = FilePairStore::open(dir.path()).unwrap();
        store.write_identity(id()).unwrap();
        let digest = store.write_effect(b"graph-effect-bytes").unwrap();
        store.write_receipt(digest).unwrap();
        let ok = store.recover().unwrap();
        assert_eq!(ok.receipt_class, ReceiptClass::DurableReceipt);
        assert_eq!(ok.effect_digest, Some(digest));
        store.truncate_effect(4).unwrap();
        let err = store.recover().unwrap_err();
        assert!(err == QdnfError::Incomplete || err == QdnfError::Conflict);
        assert!(tempfile_crash_injected());
    }

    #[test]
    fn kill_before_receipt_leaves_no_false_durable() {
        let dir = TempDir::new().unwrap();
        let store = FilePairStore::open(dir.path()).unwrap();
        store.write_identity(id()).unwrap();
        let _ = store.write_effect(b"effect-only").unwrap();
        let got = store.recover().unwrap();
        assert_eq!(got.receipt_class, ReceiptClass::Effect);
        assert_ne!(got.receipt_class, ReceiptClass::DurableReceipt);
    }

    #[test]
    fn truncated_volume_is_conflict_or_incomplete() {
        let dir = TempDir::new().unwrap();
        let store = FilePairStore::open(dir.path()).unwrap();
        store.write_identity(id()).unwrap();
        let digest = store.write_effect(b"full-effect-record").unwrap();
        store.write_receipt(digest).unwrap();
        store.truncate_effect(0).unwrap();
        let err = store.recover().unwrap_err();
        assert!(err == QdnfError::Incomplete || err == QdnfError::Conflict);
    }

    #[test]
    fn os_process_kill_qualified_is_false() {
        assert!(!os_process_kill_qualified());
        assert!(tempfile_crash_injected());
        assert!(!crate::net::peer::replication::crash::disk_backend_crash_injected());
    }
}
