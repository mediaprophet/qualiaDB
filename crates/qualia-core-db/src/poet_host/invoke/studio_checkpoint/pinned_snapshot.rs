//! Caller-budgeted Windows scratch backing for canonical checkpoint reads.
//! This layer provides byte integrity and stable backing, not authority.
use super::{validate_view, CheckpointLimits, ValidatedCheckpointView};
use memmap2::{Mmap, MmapOptions};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

const FILE_FLAG_DELETE_ON_CLOSE: u32 = 0x0400_0000;
// DELETE is required by CreateFileW with FILE_FLAG_DELETE_ON_CLOSE.
const READ_WRITE_DELETE: u32 = 0x8000_0000 | 0x4000_0000 | 0x0001_0000;
static NONCE: AtomicU64 = AtomicU64::new(0);

/// A digest-pinned, delete-on-close scratch file held with Windows share mode
/// zero. The original path can change after copying without changing this
/// snapshot. The private file handle is never exposed and no writes occur
/// after mapping. A valid snapshot does not establish source/release rights.
/// The mapping is dropped before the handle, which triggers file cleanup.
pub struct PinnedCheckpointSnapshot {
    mapping: Mmap,
    _file: File,
    digest: [u8; 32],
}

impl PinnedCheckpointSnapshot {
    /// Snapshot `source` into a unique file below the explicitly selected
    /// scratch directory. `quota` is the maximum file size, not a reservation;
    /// concurrent callers must coordinate any aggregate disk budget. The caller
    /// must provide a trusted host-owned directory with appropriate access
    /// control. Canonicalizing its path does not fence races on ancestor
    /// directories and does not establish confidentiality. A blocking file
    /// operation is not interruptible; cancellation is checked between chunks.
    /// Windows sharing and delete-on-close protect the newly created file from
    /// ordinary competing opens/replacement while the owner is alive, not from
    /// privileged raw-volume access or external hardware failures.
    pub fn create(source: &Path, scratch_dir: &Path, quota: u64, expected: [u8; 32], cancelled: &AtomicBool) -> Result<Self, String> {
        if quota == 0 || cancelled.load(Ordering::Acquire) { return Err("snapshot cancelled or quota is zero".into()); }
        let parent = scratch_dir.canonicalize().map_err(|e| format!("snapshot directory: {e}"))?;
        if !parent.is_dir() { return Err("snapshot parent is not a directory".into()); }
        let mut input = File::open(source).map_err(|e| format!("snapshot source: {e}"))?;
        let metadata = input.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.len() > quota { return Err("snapshot source exceeds quota or is not a file".into()); }
        let mut output = None;
        for _ in 0..64 {
            let id = NONCE.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!("poet-snapshot-{}-{id:016x}.hmc.tmp", std::process::id()));
            match OpenOptions::new().read(true).write(true).access_mode(READ_WRITE_DELETE).create_new(true).share_mode(0)
                .custom_flags(FILE_FLAG_DELETE_ON_CLOSE).open(path) {
                Ok(file) => { output = Some(file); break; }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(format!("snapshot scratch creation: {e}")),
            }
        }
        let mut file = output.ok_or("snapshot scratch name collision limit")?;
        let mut sha = Sha256::new();
        let mut total = 0u64;
        let mut block = [0u8; 64 * 1024];
        loop {
            if cancelled.load(Ordering::Acquire) { return Err("snapshot cancelled".into()); }
            let remaining = quota - total;
            let wanted = remaining.saturating_add(1).min(block.len() as u64) as usize;
            let n = match input.read(&mut block[..wanted]) {
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(format!("snapshot read: {e}")),
                Ok(n) => n,
            };
            if n == 0 { break; }
            if n as u64 > remaining { return Err("snapshot source grew beyond quota".into()); }
            file.write_all(&block[..n]).map_err(|e| format!("snapshot write: {e}"))?;
            sha.update(&block[..n]); total += n as u64;
        }
        if cancelled.load(Ordering::Acquire) { return Err("snapshot cancelled".into()); }
        let digest: [u8; 32] = sha.finalize().into();
        if digest != expected { return Err("snapshot source digest changed".into()); }
        if total == 0 { return Err("snapshot source is empty".into()); }
        file.flush().map_err(|e| format!("snapshot flush: {e}"))?;
        file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        // SAFETY: this file was created with create_new and share_mode(0), is
        // privately owned, and is never mutated after this mapping is created.
        // The mapping is dropped before the exclusive file handle. The caller
        // must use a trusted directory and the documented Windows assumptions.
        let mapping = unsafe { MmapOptions::new().map(&file) }.map_err(|e| format!("snapshot mapping: {e}"))?;
        if cancelled.load(Ordering::Acquire) { return Err("snapshot cancelled".into()); }
        Ok(Self { mapping, _file: file, digest })
    }

    /// Validate the exact pinned bytes using the same canonical checks as
    /// checkpoint decode. This synchronous validation is not interruptible;
    /// cancellation after it finishes withholds the result. Its returned view
    /// borrows the immutable snapshot and cannot outlive this owner.
    pub fn validate(&self, limits: CheckpointLimits, cancelled: &AtomicBool) -> Result<ValidatedCheckpointView<'_>, String> {
        if cancelled.load(Ordering::Acquire) { return Err("snapshot validation cancelled".into()); }
        let view = validate_view(&self.mapping, limits)?;
        if cancelled.load(Ordering::Acquire) { return Err("snapshot validation cancelled".into()); }
        if view.file_digest() != self.digest { return Err("snapshot digest changed during validation".into()); }
        Ok(view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[test]
    fn pinned_scratch_is_exclusive_scoped_and_digest_checked() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.hmc");
        fs::write(&source, b"immutable fixture").unwrap();
        let digest: [u8; 32] = Sha256::digest(b"immutable fixture").into();
        let cancelled = AtomicBool::new(false);
        assert!(PinnedCheckpointSnapshot::create(&source, dir.path(), 4, digest, &cancelled).is_err());
        assert!(PinnedCheckpointSnapshot::create(&source, dir.path(), 100, [0; 32], &cancelled).is_err());
        let snapshot = PinnedCheckpointSnapshot::create(&source, dir.path(), 100, digest, &cancelled).unwrap();
        let scratch = fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().path()).find(|p| p != &source).unwrap();
        assert!(OpenOptions::new().write(true).open(&scratch).is_err());
        fs::write(&source, b"changed original").unwrap();
        assert_eq!(&snapshot.mapping[..], b"immutable fixture");
        assert!(snapshot.validate(CheckpointLimits::default(), &cancelled).is_err()); // not HMC
        drop(snapshot);
        assert!(!scratch.exists());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        cancelled.store(true, Ordering::Release);
        assert!(PinnedCheckpointSnapshot::create(&source, dir.path(), 100, digest, &cancelled).is_err());
    }

    #[test]
    fn canonical_admission_borrows_pinned_original_after_source_change() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.hmc");
        let id = format!("urn:checkpoint:pinned:{}", std::process::id());
        let (_, stamp) = super::super::super::create_artifact_if_absent(
            super::super::super::StudioArtifact::new(&id, "Geometry", "Pinned fixture")
        ).unwrap();
        let limits = CheckpointLimits::default();
        let checkpoint = super::super::capture(&id, &stamp, limits).unwrap();
        let bytes = super::super::encode(&checkpoint, vec![], limits).unwrap();
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        fs::write(&source, &bytes).unwrap();
        let cancelled = AtomicBool::new(false);
        let snapshot = PinnedCheckpointSnapshot::create(&source, dir.path(), bytes.len() as u64, digest, &cancelled).unwrap();
        fs::write(&source, b"new revision").unwrap();
        let view = snapshot.validate(limits, &cancelled).unwrap();
        assert_eq!(view.artifact().id, id);
        assert_eq!(view.file_digest(), digest);
        drop(view);
        cancelled.store(true, Ordering::Release);
        assert!(snapshot.validate(limits, &cancelled).is_err());
    }
}
