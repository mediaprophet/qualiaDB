//! Caller-selected native persistence. No default workspace/project destination.
use super::{codec::digest, decode, Checkpoint, CheckpointLimits};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug)]
pub enum ExpectedFile {
    Absent,
    Sha256([u8; 32]),
}
#[derive(Debug)]
pub struct SaveReceipt {
    pub path: PathBuf,
    pub digest: [u8; 32],
    pub bytes: usize,
    /// Installation committed. A directory sync failure affects power-loss
    /// durability, not the success of the atomic file replacement.
    pub durability_warning: Option<String>,
}

fn destination(path: &Path) -> Result<PathBuf, String> {
    let name = path
        .file_name()
        .ok_or("checkpoint destination needs a file name")?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent.canonicalize().map_err(|e| e.to_string())?;
    let path = parent.join(name);
    match fs::symlink_metadata(&path) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_file() => {
            Err("checkpoint destination must be a regular file, never a link/directory".into())
        }
        Ok(_) => Ok(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(path),
        Err(e) => Err(e.to_string()),
    }
}

fn read_bounded(path: &Path, maximum: usize) -> Result<Vec<u8>, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > maximum as u64 {
        return Err("checkpoint input is not a regular file within caller budget".into());
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((maximum as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > maximum {
        return Err("checkpoint input grew beyond caller budget".into());
    }
    Ok(bytes)
}

pub fn load(path: &Path, limits: CheckpointLimits) -> Result<Checkpoint, String> {
    let path = destination(path)?;
    decode(&read_bounded(&path, limits.file_bytes)?, limits)
}

// Check between bounded reads; a blocking filesystem call and the canonical
// decoder cannot be interrupted. Nothing is returned after cancellation.
fn read_cancellable(mut reader: impl Read, maximum: usize, cancelled: &AtomicBool) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 64 * 1024];
    loop {
        if cancelled.load(Ordering::Acquire) { return Err("checkpoint load cancelled".into()); }
        // Read one extra byte only after reaching the budget to detect growth.
        let wanted = maximum.saturating_sub(bytes.len()).saturating_add(1).min(chunk.len());
        let n = match reader.read(&mut chunk[..wanted]) {
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.to_string()),
            Ok(n) => n,
        };
        if n == 0 { break; }
        if n > maximum.saturating_sub(bytes.len()) { return Err("checkpoint input grew beyond caller budget".into()); }
        bytes.try_reserve(n).map_err(|_| "checkpoint input allocation refused")?;
        bytes.extend_from_slice(&chunk[..n]);
    }
    if cancelled.load(Ordering::Acquire) { return Err("checkpoint load cancelled".into()); }
    Ok(bytes)
}

/// Canonical checkpoint admission with cooperative cancellation during file
/// reading. The decoder remains synchronous; cancellation after decoding
/// prevents returning its result. This does not authorize access or establish
/// a stable file revision on its own.
pub fn load_cancellable(path: &Path, limits: CheckpointLimits, cancelled: &AtomicBool) -> Result<Checkpoint, String> {
    if cancelled.load(Ordering::Acquire) { return Err("checkpoint load cancelled".into()); }
    let path = destination(path)?;
    let file = File::open(path).map_err(|e| e.to_string())?;
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.len() > limits.file_bytes as u64 {
        return Err("checkpoint input is not a regular file within caller budget".into());
    }
    let bytes = read_cancellable(file, limits.file_bytes, cancelled)?;
    let checkpoint = decode(&bytes, limits)?;
    if cancelled.load(Ordering::Acquire) { return Err("checkpoint load cancelled".into()); }
    Ok(checkpoint)
}

#[cfg(test)]
mod cancellable_read_tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn exact_budget_is_accepted_and_growth_is_refused() {
        let cancel = AtomicBool::new(false);
        assert_eq!(read_cancellable(Cursor::new(b"abc"), 3, &cancel).unwrap(), b"abc");
        assert!(read_cancellable(Cursor::new(b"abcd"), 3, &cancel).is_err());
        assert!(read_cancellable(Cursor::new(b""), 0, &cancel).is_ok());
        assert!(read_cancellable(Cursor::new(b"x"), 0, &cancel).is_err());
    }

    #[test]
    fn cancellation_between_reads_withholds_partial_bytes() {
        struct Cancelling<'a>(&'a AtomicBool);
        impl Read for Cancelling<'_> {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                buf[0] = 42;
                self.0.store(true, Ordering::Release);
                Ok(1)
            }
        }
        let cancel = AtomicBool::new(false);
        assert_eq!(read_cancellable(Cancelling(&cancel), 10, &cancel).unwrap_err(), "checkpoint load cancelled");
        assert!(read_cancellable(Cursor::new(b""), 10, &cancel).is_err());
    }
}

/// Native bounded read retaining the exact verified original HMC. Inert read
/// compatibility does not grant permission to activate or evaluate source.
pub fn load_source_carrier(
    path: &Path,
    limits: CheckpointLimits,
) -> Result<super::VerifiedSourceCarrier, String> {
    let path = destination(path)?;
    super::decode_source_carrier(&read_bounded(&path, limits.file_bytes)?, limits)
}

fn check_expected(path: &Path, expected: ExpectedFile, budget: usize) -> Result<(), String> {
    match expected {
        ExpectedFile::Absent => match fs::symlink_metadata(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            _ => Err("checkpoint destination already exists or cannot be inspected".into()),
        },
        ExpectedFile::Sha256(expected) => {
            destination(path)?;
            if digest(&read_bounded(path, budget)?) != expected {
                return Err(
                    "checkpoint destination digest changed; existing bytes preserved".into(),
                );
            }
            Ok(())
        }
    }
}

fn persist_failure(
    path: &Path,
    staged: &Path,
    attempts: usize,
    first: &str,
    last: &str,
    reason: &str,
) -> String {
    format!("checkpoint persist did not commit ({reason}); destination {}; staged {}; attempts {attempts}; first persist error: {first}; last persist error: {last}", path.display(), staged.display())
}

/// Cold Windows installation can encounter temporary deny-delete/lock errors.
/// Retain this exact staged file and the caller's writer lock, with at most
/// 315 ms of retry admission/wait time. Synchronous filesystem IO is not timed
/// out by this budget. Every further attempt rechecks the destination digest.
/// The observer only lets tests release a real deny-delete handle AFTER an
/// actual failed OS persist; production passes a no-op. No global test hooks.
pub(super) fn persist_staged(
    mut staged: tempfile::NamedTempFile,
    path: &Path,
    expected: ExpectedFile,
    budget: usize,
    mut on_retry: impl FnMut(usize),
) -> Result<(File, usize), String> {
    const PAUSES_MS: [u64; 6] = [5, 10, 20, 40, 80, 160];
    let started = Instant::now();
    let retry_budget = Duration::from_millis(315);
    let staged_path = staged.path().to_path_buf();
    let mut attempts = 0;
    let mut first_error = String::new();
    let mut last_error = String::new();
    loop {
        if let Err(error) = check_expected(path, expected, budget) {
            return Err(persist_failure(
                path,
                &staged_path,
                attempts,
                &first_error,
                &last_error,
                &format!("expected destination guard refused: {error}"),
            ));
        }
        if attempts > 0 && started.elapsed() >= retry_budget {
            return Err(persist_failure(
                path,
                &staged_path,
                attempts,
                &first_error,
                &last_error,
                "retry budget exhausted",
            ));
        }
        attempts += 1;
        let result = match expected {
            ExpectedFile::Absent => staged.persist_noclobber(path),
            ExpectedFile::Sha256(_) => staged.persist(path),
        };
        match result {
            Ok(installed) => return Ok((installed, attempts)),
            Err(failed) => {
                let retryable =
                    cfg!(windows) && matches!(failed.error.raw_os_error(), Some(5 | 32 | 33));
                last_error = failed.error.to_string();
                if first_error.is_empty() {
                    first_error = last_error.clone();
                }
                staged = failed.file;
                if !retryable || attempts > PAUSES_MS.len() || started.elapsed() >= retry_budget {
                    let reason = if retryable {
                        "retry budget exhausted"
                    } else {
                        "nonretryable OS error"
                    };
                    return Err(persist_failure(
                        path,
                        &staged_path,
                        attempts,
                        &first_error,
                        &last_error,
                        reason,
                    ));
                }
                on_retry(attempts);
                let remaining = retry_budget.saturating_sub(started.elapsed());
                if !remaining.is_zero() {
                    std::thread::sleep(
                        Duration::from_millis(PAUSES_MS[attempts - 1]).min(remaining),
                    );
                }
            }
        }
    }
}

/// Validate before filesystem mutation, stage/sync a same-directory RAII temp,
/// recheck expected bytes, then atomically install. Create never clobbers.
/// Replacement serializes cooperating writers through a stable OS-locked
/// sidecar. Arbitrary external writers must honour that lock; an OS filesystem
/// has no portable atomic compare-digest-and-rename primitive. Both checks fail
/// closed on observed changes; the final outside-writer race is not concealed.
/// File bytes are synced. Windows power-loss directory durability is unqualified.
pub fn save(
    path: &Path,
    bytes: &[u8],
    expected: ExpectedFile,
    limits: CheckpointLimits,
) -> Result<SaveReceipt, String> {
    decode(bytes, limits)?;
    save_bytes(path,bytes,expected,limits.file_bytes)
}

/// Canonical bundle integrity only, not Studio checkpoint/schema equivalence,
/// source authority, rights or embodiment acceptance. The native owner supplies
/// the format-specific admission and guards the real package commit.
pub fn save_bundle(path:&Path,bytes:&[u8],expected:ExpectedFile,max_bytes:usize)->Result<SaveReceipt,String> {
    if bytes.len()>max_bytes {return Err("Bundle exceeds caller file budget".into());}
    crate::bundle::BundleReader::parse_with_limits(bytes,crate::bundle::BundleReadLimits { file_bytes:max_bytes, ..Default::default() }).map_err(|e|format!("Bundle integrity refused: {e:?}"))?;
    save_bytes(path,bytes,expected,max_bytes)
}

pub fn load_bundle_cancellable(path:&Path,max_bytes:usize,cancelled:&AtomicBool)->Result<Vec<u8>,String> {
    if cancelled.load(Ordering::Acquire){return Err("Bundle load cancelled".into());}
    let path=destination(path)?;
    let file=File::open(path).map_err(|e|e.to_string())?;
    let meta=file.metadata().map_err(|e|e.to_string())?;
    if !meta.is_file() || meta.len()>max_bytes as u64 {return Err("Bundle input exceeds caller file budget".into());}
    let bytes=read_cancellable(file,max_bytes,cancelled)?;
    crate::bundle::BundleReader::parse_with_limits(&bytes,crate::bundle::BundleReadLimits { file_bytes:max_bytes, ..Default::default() }).map_err(|e|format!("Bundle integrity refused: {e:?}"))?;
    if cancelled.load(Ordering::Acquire){return Err("Bundle load cancelled".into());}
    Ok(bytes)
}

fn save_bytes(path:&Path,bytes:&[u8],expected:ExpectedFile,max_bytes:usize)->Result<SaveReceipt,String> {
    let path = destination(path)?;
    let parent = path
        .parent()
        .ok_or("checkpoint destination parent absent")?;
    let mut lock_name = path
        .file_name()
        .ok_or("destination file name absent")?
        .to_os_string();
    lock_name.push(".poet-source.lock");
    let lock_path = parent.join(lock_name);
    if fs::symlink_metadata(&lock_path)
        .is_ok_and(|meta| meta.file_type().is_symlink() || !meta.is_file())
    {
        return Err("checkpoint lock path is not a regular file".into());
    }
    // Persistent coordination artifact, not stale temporary state. OS releases
    // its lock on process exit; never unlink it while another writer may wait.
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| e.to_string())?;
    lock.try_lock()
        .map_err(|e| format!("checkpoint writer busy: {e}"))?;
    check_expected(&path, expected, max_bytes)?;
    let mut staged = tempfile::Builder::new()
        .prefix(".poet-source-")
        .tempfile_in(parent)
        .map_err(|e| e.to_string())?;
    staged.write_all(bytes).map_err(|e| e.to_string())?;
    staged.as_file().sync_all().map_err(|e| e.to_string())?;
    let (_installed, _attempts) =
        persist_staged(staged, &path, expected, max_bytes, |_| {})?;
    // The staged file was already synced before install. No fallible operation
    // after this commit point may report a rolled-back/failed save.
    #[cfg(unix)]
    let durability_warning = File::open(parent)
        .and_then(|directory| directory.sync_all())
        .err().map(|e| format!("checkpoint installed; directory sync failed, power-loss durability unqualified: {e}"));
    #[cfg(not(unix))]
    let durability_warning = None;
    Ok(SaveReceipt {
        path,
        digest: digest(bytes),
        bytes: bytes.len(),
        durability_warning,
    })
}
