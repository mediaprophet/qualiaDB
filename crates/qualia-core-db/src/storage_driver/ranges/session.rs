use super::types::*;
use crate::{
    sensing::local_custody::{quota::StagingCharge, validate_root},
    storage_driver::MmapDriver,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    marker::PhantomData,
    path::{Path, PathBuf},
};
use tempfile::{Builder, TempDir};

/// Synchronous caller-buffered staging. Bytes and sync observations carry no
/// proof, availability, serving permission or durable-publication authority.
/// Hold the canonical runtime StorageStage while doing these operations;
/// the host owns fresh policy/time checks and cancellation/worker drain.
pub struct RangeSession<'driver, 'root> {
    // Option extraction lets consuming close and Drop share guarded cleanup.
    files: Option<[File; 3]>,
    dir: Option<TempDir>,
    parent: PathBuf,
    charge: Option<StagingCharge<'root>>,
    limits: RangeLimits,
    driver: PhantomData<&'driver MmapDriver>,
}

impl MmapDriver {
    /// Cold bounded construction under the existing driver/quota root.
    /// The charge is consumed even if preflight, creation or cleanup fails.
    /// No filesystem mutation is performed until limits and root match.
    pub fn create_range_staging<'driver, 'root>(
        &'driver self,
        charge: StagingCharge<'root>,
        limits: RangeLimits,
    ) -> Result<RangeSession<'driver, 'root>, RangeCreateError<'root>> {
        self.create_staging_inner(charge, limits, |_| {})
    }
    fn create_staging_inner<'driver, 'root>(
        &'driver self,
        charge: StagingCharge<'root>,
        limits: RangeLimits,
        before_open: impl FnOnce(&Path),
    ) -> Result<RangeSession<'driver, 'root>, RangeCreateError<'root>> {
        let fail = |error| RangeCreateError {
            error,
            cleanup_pending: None,
        };
        let total = limits.total().map_err(fail)?;
        if total > charge.allowance() {
            return Err(fail(RangeError::InvalidLimits));
        }
        if !cfg!(any(unix, windows)) {
            return Err(fail(RangeError::Unsupported));
        }
        validate_root(&self.root).map_err(|_| fail(RangeError::InvalidRoot))?;
        validate_root(charge.root()).map_err(|_| fail(RangeError::InvalidRoot))?;
        let parent = checked_root(&self.root).map_err(fail)?;
        let quota_root = checked_root(charge.root()).map_err(fail)?;
        if parent != quota_root {
            return Err(fail(RangeError::ScopeMismatch));
        }
        let dir = Builder::new()
            .prefix("qualia-range-")
            .tempdir_in(&parent)
            .map_err(|e| fail(e.into()))?;
        before_open(dir.path());
        let opened = prepare_directory(&dir)
            .and_then(|_| write_owner(dir.path(), charge.attempt_id()))
            .and_then(|_| open_files(dir.path()));
        match opened {
            Ok(files) => Ok(RangeSession {
                files: Some(files),
                dir: Some(dir),
                parent,
                charge: Some(charge),
                limits,
                driver: PhantomData,
            }),
            Err(error) => Err(RangeCreateError {
                error,
                cleanup_pending: close_directory(dir, parent, charge).err(),
            }),
        }
    }
}
pub(super) fn checked_root(path: &Path) -> Result<PathBuf, RangeError> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_dir() || meta.file_type().is_symlink() || is_reparse(&meta) {
        return Err(RangeError::InvalidRoot);
    }
    Ok(fs::canonicalize(path)?)
}
fn prepare_directory(dir: &TempDir) -> Result<(), RangeError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700))?;
    }
    let _ = dir;
    Ok(())
}
fn open_files(dir: &Path) -> Result<[File; 3], RangeError> {
    fn open(dir: &Path, which: RangeFile) -> Result<File, RangeError> {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(0);
        }
        let file = options.open(dir.join(which.name()))?;
        file.try_lock().map_err(|_| RangeError::LockUnavailable)?;
        Ok(file)
    }
    // On an intermediate error, already opened handles drop before cleanup.
    Ok([
        open(dir, RangeFile::Payload)?,
        open(dir, RangeFile::Proof)?,
        open(dir, RangeFile::Checkpoint)?,
    ])
}
impl RangeSession<'_, '_> {
    pub const fn limits(&self) -> RangeLimits {
        self.limits
    }
    pub fn attempt_id(&self) -> [u8; 32] {
        self.charge
            .as_ref()
            .expect("live range charge")
            .attempt_id()
    }
    pub fn staging_path(&self) -> &Path {
        self.dir.as_ref().expect("live range directory").path()
    }
    pub const fn capabilities(&self) -> RangeCapabilities {
        RangeCapabilities {
            writer_exclusion: if cfg!(windows) {
                WriterExclusion::WindowsShareDeny
            } else {
                WriterExclusion::CooperativeLockOnly
            },
            directory_power_loss_qualified: false,
        }
    }
    /// Observes length through the retained handle, including sparse holes.
    /// This diagnostic is never a verified-presence or durability map.
    pub fn observed_file_length(&self, which: RangeFile) -> Result<u64, RangeError> {
        Ok(
            self.files.as_ref().expect("live range handles")[which.index()]
                .metadata()?
                .len(),
        )
    }
    /// Reads exactly the caller's slice. Sparse zero bytes are not presence.
    pub fn read_exact_at(
        &mut self,
        which: RangeFile,
        offset: u64,
        out: &mut [u8],
    ) -> Result<(), RangeError> {
        self.limits.check(which, offset, out.len())?;
        if out.is_empty() {
            return Ok(());
        }
        let file = &mut self.files.as_mut().expect("live range handles")[which.index()];
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(out)?;
        Ok(())
    }
    /// Does not allocate, preallocate the ceiling, or accept a filename.
    /// An I/O error may have written a prefix; no atomicity is promised.
    pub fn write_exact_at(
        &mut self,
        which: RangeFile,
        offset: u64,
        bytes: &[u8],
    ) -> Result<(), RangeError> {
        self.limits.check(which, offset, bytes.len())?;
        if bytes.is_empty() {
            return Ok(());
        }
        let file = &mut self.files.as_mut().expect("live range handles")[which.index()];
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(bytes)?;
        Ok(())
    }
    /// Actual file sync completion, not directory durability or verification.
    pub fn sync_file(&self, which: RangeFile) -> Result<FileBarrier, RangeError> {
        self.files.as_ref().expect("live range handles")[which.index()].sync_all()?;
        Ok(FileBarrier::SyncAllReturned)
    }
}
impl<'root> RangeSession<'_, 'root> {
    pub fn close(self) -> Result<CleanupObservation, CleanupPending<'root>> {
        self.close_then(|| {})
    }
    fn close_then(
        mut self,
        after_handles: impl FnOnce(),
    ) -> Result<CleanupObservation, CleanupPending<'root>> {
        drop(self.files.take());
        after_handles();
        let dir = self.dir.take().expect("live range directory");
        let parent = std::mem::take(&mut self.parent);
        let charge = self.charge.take().expect("live range charge");
        close_directory(dir, parent, charge)
    }
}
impl Drop for RangeSession<'_, '_> {
    fn drop(&mut self) {
        drop(self.files.take());
        if let Some(dir) = self.dir.take() {
            if let Some(charge) = self.charge.take() {
                // Drop cannot return a pending owner. Failed checked cleanup
                // leaves charged artifacts; explicit close preserves diagnosis.
                let _ = close_directory(dir, std::mem::take(&mut self.parent), charge);
            } else {
                // Defensive: without the admitted attempt, never delete.
                let _ = dir.keep();
            }
        }
    }
}

fn close_directory<'root>(
    dir: TempDir,
    parent: PathBuf,
    charge: StagingCharge<'root>,
) -> Result<CleanupObservation, CleanupPending<'root>> {
    let path = dir.path().to_path_buf();
    if let Err(error) =
        check_owned_child(&path, &parent).and_then(|_| check_owner(&path, charge.attempt_id()))
    {
        // Prevent TempDir Drop from bypassing a failed namespace guard.
        let _ = dir.keep();
        return Err(CleanupPending {
            path,
            parent,
            charge,
            error,
        });
    }
    match dir.close() {
        Ok(()) => Ok(CleanupObservation::Removed),
        Err(error) => {
            let original = RangeError::from(error);
            let error = restore_owner_after_cleanup_error(&path, &parent, charge.attempt_id())
                .err()
                .unwrap_or(original);
            Err(CleanupPending {
                path,
                parent,
                charge,
                error,
            })
        }
    }
}
impl CleanupPending<'_> {
    /// Retries only this internally owned canonical child. Root protection
    /// against concurrent namespace replacement is a host precondition.
    /// Failure retains ownership; dropping a pending owner leaves charged
    /// uncertain artifacts for host recovery, and never reports removal.
    pub fn retry(mut self) -> Result<CleanupObservation, Self> {
        if let Err(error) = check_owned_child(&self.path, &self.parent)
            .and_then(|_| check_owner(&self.path, self.charge.attempt_id()))
        {
            self.error = error;
            return Err(self);
        }
        match fs::remove_dir_all(&self.path) {
            Ok(()) => Ok(CleanupObservation::Removed),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(CleanupObservation::Removed),
            Err(e) => {
                let original = RangeError::from(e);
                self.error = restore_owner_after_cleanup_error(
                    &self.path,
                    &self.parent,
                    self.charge.attempt_id(),
                )
                .err()
                .unwrap_or(original);
                Err(self)
            }
        }
    }
}
pub(super) fn check_owned_child(path: &Path, parent: &Path) -> Result<(), RangeError> {
    if !path.is_absolute()
        || path.parent() != Some(parent)
        || !path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("qualia-range-") && n.len() > 13)
        || fs::canonicalize(parent)? != parent
    {
        return Err(RangeError::InvalidRoot);
    }
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    if !meta.is_dir()
        || meta.file_type().is_symlink()
        || is_reparse(&meta)
        || fs::canonicalize(path)? != path
    {
        return Err(RangeError::InvalidRoot);
    }
    Ok(())
}
pub(super) fn is_reparse(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        let _ = meta;
        false
    }
}
#[cfg(test)]
pub(super) fn close_for_test<'root>(
    session: RangeSession<'_, 'root>,
    after_handles: impl FnOnce(),
) -> Result<CleanupObservation, CleanupPending<'root>> {
    session.close_then(after_handles)
}

#[cfg(test)]
pub(super) fn creation_failure_for_test<'driver, 'root>(
    driver: &'driver MmapDriver,
    charge: StagingCharge<'root>,
    limits: RangeLimits,
) -> Result<RangeSession<'driver, 'root>, RangeCreateError<'root>> {
    driver.create_staging_inner(charge, limits, |path| {
        File::create(path.join("proof")).unwrap();
    })
}
const OWNER_NAME: &str = ".owner";
const OWNER_BYTES: usize = 64;
const OWNER_MAGIC: &[u8; 8] = b"QBSOWN1\0";
pub(super) fn owner_bytes(attempt: [u8; 32]) -> [u8; OWNER_BYTES] {
    let mut bytes = [0; OWNER_BYTES];
    bytes[..8].copy_from_slice(OWNER_MAGIC);
    bytes[8..10].copy_from_slice(&1u16.to_be_bytes());
    bytes[16..48].copy_from_slice(&attempt);
    bytes
}
fn write_owner(dir: &Path, attempt: [u8; 32]) -> Result<(), RangeError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
    }
    let mut file = options.open(dir.join(OWNER_NAME))?;
    file.write_all(&owner_bytes(attempt))?;
    Ok(())
}
fn check_owner(dir: &Path, attempt: [u8; 32]) -> Result<(), RangeError> {
    // Absence of the entire owned child needs no recursive deletion.
    if let Err(e) = fs::symlink_metadata(dir) {
        if e.kind() == std::io::ErrorKind::NotFound {
            return Ok(());
        }
        return Err(e.into());
    }
    let path = dir.join(OWNER_NAME);
    let meta = fs::symlink_metadata(&path).map_err(|_| RangeError::OwnershipMarker)?;
    if !meta.is_file()
        || meta.file_type().is_symlink()
        || is_reparse(&meta)
        || meta.len() != OWNER_BYTES as u64
    {
        return Err(RangeError::OwnershipMarker);
    }
    let mut file = File::open(path).map_err(|_| RangeError::OwnershipMarker)?;
    let opened = file.metadata().map_err(|_| RangeError::OwnershipMarker)?;
    if !opened.is_file() || is_reparse(&opened) || opened.len() != OWNER_BYTES as u64 {
        return Err(RangeError::OwnershipMarker);
    }
    let mut bytes = [0; OWNER_BYTES];
    file.read_exact(&mut bytes)
        .map_err(|_| RangeError::OwnershipMarker)?;
    let mut trailing = [0; 1];
    if bytes != owner_bytes(attempt)
        || file
            .read(&mut trailing)
            .map_err(|_| RangeError::OwnershipMarker)?
            != 0
    {
        return Err(RangeError::OwnershipMarker);
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn drop_for_test(session: RangeSession<'_, '_>, after_handles: impl FnOnce()) {
    let mut session = session;
    drop(session.files.take());
    after_handles();
    drop(session);
}

fn restore_owner_after_cleanup_error(
    path: &Path,
    parent: &Path,
    attempt: [u8; 32],
) -> Result<(), RangeError> {
    // Only called after THIS operation verified exact ownership and then its
    // native removal failed. Protected-root namespace stability is required.
    // Never use this to repair an initial missing/malformed/untrusted marker.
    check_owned_child(path, parent)?;
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
        Ok(_) => {}
    }
    match fs::symlink_metadata(path.join(OWNER_NAME)) {
        Ok(_) => return check_owner(path, attempt), // Never overwrite anything.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    match write_owner(path, attempt) {
        Ok(()) => Ok(()),
        // A newly appeared existing marker must match exactly; no overwrite.
        Err(RangeError::Io(std::io::ErrorKind::AlreadyExists)) => check_owner(path, attempt),
        Err(e) => Err(e),
    }
}
#[cfg(test)]
pub(super) fn restore_owner_for_test(
    path: &Path,
    parent: &Path,
    attempt: [u8; 32],
) -> Result<(), RangeError> {
    restore_owner_after_cleanup_error(path, parent, attempt)
}
