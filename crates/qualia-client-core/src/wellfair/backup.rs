//! Portable **backup / restore** of the WellFair data subtree (T3.3 release-hardening slice).
//!
//! A backup is a single archive — `lz4(cbor(BackupArchive))` — of every file under
//! `<storage_root>/wellfair/`: the journal, receipts, sync outbox/inbox, consent + blob stores, and
//! the encrypted Sanctuary vault. The Sanctuary vault is already AEAD-encrypted at rest, so it stays
//! encrypted inside the archive; the rest carries the same at-rest posture as on disk (an optional
//! passphrase wrapper over the whole archive is a clean follow-up).
//!
//! Restore is **path-traversal-safe**: every archive key must be a relative path whose components are
//! all `Normal` (no `..`, no root, no drive prefix), so a malicious archive can never write outside
//! the target `wellfair/` directory.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

const BACKUP_VERSION: u16 = 1;
const WELLFAIR_SUBDIR: &str = "wellfair";

/// The decoded archive: a version + creation stamp + a map of relative path → file bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupArchive {
    pub version: u16,
    pub created_unix: u32,
    /// Forward-slash relative paths under `wellfair/` → file contents.
    pub files: BTreeMap<String, Vec<u8>>,
}

/// A count of what an export/import moved.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupReport {
    pub files: usize,
    pub bytes: u64,
}

fn collect_files(
    base: &Path,
    dir: &Path,
    out: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        if file_type.is_dir() {
            collect_files(base, &path, out)?;
        } else if file_type.is_file() {
            let rel = path.strip_prefix(base).map_err(|e| e.to_string())?;
            let key = rel
                .components()
                .filter_map(|c| match c {
                    Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("/");
            if key.is_empty() {
                continue;
            }
            let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            out.insert(key, bytes);
        }
    }
    Ok(())
}

/// Build an in-memory archive of the `wellfair/` subtree under `storage_root`.
pub fn build_archive(storage_root: &Path, created_unix: u32) -> Result<BackupArchive, String> {
    let base = storage_root.join(WELLFAIR_SUBDIR);
    let mut files = BTreeMap::new();
    if base.exists() {
        collect_files(&base, &base, &mut files)?;
    }
    Ok(BackupArchive {
        version: BACKUP_VERSION,
        created_unix,
        files,
    })
}

/// Encode an archive to its on-disk form: `lz4(cbor(archive))`.
pub fn encode_archive(archive: &BackupArchive) -> Result<Vec<u8>, String> {
    let mut cbor = Vec::new();
    ciborium::into_writer(archive, &mut cbor).map_err(|e| e.to_string())?;
    Ok(lz4_flex::compress_prepend_size(&cbor))
}

/// Decode an archive from its on-disk form.
pub fn decode_archive(bytes: &[u8]) -> Result<BackupArchive, String> {
    let cbor = lz4_flex::decompress_size_prepended(bytes).map_err(|e| e.to_string())?;
    let archive: BackupArchive = ciborium::from_reader(&cbor[..]).map_err(|e| e.to_string())?;
    if archive.version != BACKUP_VERSION {
        return Err(format!(
            "unsupported backup version {} (expected {BACKUP_VERSION})",
            archive.version
        ));
    }
    Ok(archive)
}

/// Join a relative archive key under `base`, **rejecting** any non-`Normal` component (path-traversal
/// defense: no `..`, no absolute/root, no drive prefix).
fn safe_join(base: &Path, rel: &str) -> Result<PathBuf, String> {
    let mut out = base.to_path_buf();
    for comp in Path::new(rel).components() {
        match comp {
            Component::Normal(c) => out.push(c),
            _ => return Err(format!("unsafe path in backup archive: '{rel}'")),
        }
    }
    Ok(out)
}

/// Restore an archive into the `wellfair/` subtree under `storage_root` (creating directories).
pub fn restore_archive(
    storage_root: &Path,
    archive: &BackupArchive,
) -> Result<BackupReport, String> {
    let base = storage_root.join(WELLFAIR_SUBDIR);
    let mut report = BackupReport::default();
    for (rel, bytes) in &archive.files {
        let target = safe_join(&base, rel)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&target, bytes).map_err(|e| e.to_string())?;
        report.files += 1;
        report.bytes += bytes.len() as u64;
    }
    Ok(report)
}

/// Convenience: build + encode a backup of `storage_root`'s WellFair data.
pub fn create_backup(storage_root: &Path, created_unix: u32) -> Result<Vec<u8>, String> {
    encode_archive(&build_archive(storage_root, created_unix)?)
}

/// Convenience: decode + restore a backup into `storage_root`.
pub fn restore_backup(storage_root: &Path, bytes: &[u8]) -> Result<BackupReport, String> {
    restore_archive(storage_root, &decode_archive(bytes)?)
}

fn stat_dir(dir: &Path, files: &mut usize, bytes: &mut u64) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        if file_type.is_dir() {
            stat_dir(&entry.path(), files, bytes)?;
        } else if file_type.is_file() {
            *files += 1;
            *bytes += entry.metadata().map_err(|e| e.to_string())?.len();
        }
    }
    Ok(())
}

/// Cheap `(file count, total bytes)` of the `wellfair/` data subtree — metadata only, no file reads.
pub fn wellfair_data_stats(storage_root: &Path) -> Result<(usize, u64), String> {
    let base = storage_root.join(WELLFAIR_SUBDIR);
    let mut files = 0;
    let mut bytes = 0u64;
    if base.exists() {
        stat_dir(&base, &mut files, &mut bytes)?;
    }
    Ok((files, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, contents: &[u8]) {
        let p = root.join(WELLFAIR_SUBDIR).join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, contents).unwrap();
    }

    #[test]
    fn backup_round_trips_the_wellfair_subtree() {
        let src = tempfile::tempdir().unwrap();
        write(src.path(), "journal.jsonl", b"{\"kind\":\"weight\"}\n");
        write(
            src.path(),
            "sanctuary_vault.cbor",
            &[0xDE, 0xAD, 0xBE, 0xEF],
        );
        write(src.path(), "blobs/aa/bb.bin", b"blobbytes");

        let bytes = create_backup(src.path(), 1_700_000_000).unwrap();

        // Restore into a fresh, empty root.
        let dst = tempfile::tempdir().unwrap();
        let report = restore_backup(dst.path(), &bytes).unwrap();
        assert_eq!(report.files, 3);

        // Every file is byte-identical after the round trip.
        let base = dst.path().join(WELLFAIR_SUBDIR);
        assert_eq!(
            fs::read(base.join("journal.jsonl")).unwrap(),
            b"{\"kind\":\"weight\"}\n"
        );
        assert_eq!(
            fs::read(base.join("sanctuary_vault.cbor")).unwrap(),
            vec![0xDE, 0xAD, 0xBE, 0xEF]
        );
        assert_eq!(
            fs::read(base.join("blobs/aa/bb.bin")).unwrap(),
            b"blobbytes"
        );
    }

    #[test]
    fn empty_root_yields_empty_archive() {
        let src = tempfile::tempdir().unwrap();
        let archive = build_archive(src.path(), 1).unwrap();
        assert!(archive.files.is_empty());
        // Encode/decode of an empty archive still round-trips.
        let bytes = encode_archive(&archive).unwrap();
        assert_eq!(decode_archive(&bytes).unwrap(), archive);
    }

    #[test]
    fn restore_rejects_path_traversal_keys() {
        let mut files = BTreeMap::new();
        files.insert("../escape.txt".to_string(), b"evil".to_vec());
        let archive = BackupArchive {
            version: BACKUP_VERSION,
            created_unix: 1,
            files,
        };
        let dst = tempfile::tempdir().unwrap();
        assert!(restore_archive(dst.path(), &archive).is_err());
        // Nothing escaped the target directory.
        assert!(!dst.path().join("escape.txt").exists());
    }

    #[test]
    fn decode_rejects_wrong_version() {
        let archive = BackupArchive {
            version: 999,
            created_unix: 1,
            files: BTreeMap::new(),
        };
        let bytes = encode_archive(&archive).unwrap();
        assert!(decode_archive(&bytes).is_err());
    }

    #[test]
    fn decode_rejects_garbage() {
        assert!(decode_archive(b"not an archive at all").is_err());
    }
}
