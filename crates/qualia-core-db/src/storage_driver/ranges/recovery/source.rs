use super::super::session::{check_owned_child, owner_bytes};
use super::locator::{canonical_root, map_range};
use super::types::*;
use crate::{
    sensing::local_custody::{
        open_regular_exclusive,
        quota::recovery::{RecoveryRootScope, RecoveryScopeKey},
    },
    storage_driver::{
        ranges::{RangeCapabilities, RangeFile, RangeLimits, WriterExclusion},
        MmapDriver,
    },
};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

/// Untrusted read-only material. No TempDir adoption/deletion or recovered quota.
pub struct RangeRecoverySource<'root> {
    files: [Option<File>; 3],
    _marker: File,
    _root: &'root Path,
    canonical: [u8; MAX_ROOT_BYTES],
    root_len: u16,
    scope: RecoveryScopeKey,
    snapshot: [u8; 32],
    attempt: [u8; 32],
    limits: RangeLimits,
}
impl MmapDriver {
    pub(crate) fn open_range_recovery_source<'root>(
        &self,
        scope: &RecoveryRootScope<'root>,
        locator: StagingLocator,
        limits: RangeLimits,
    ) -> Result<RangeRecoverySource<'root>, RecoveryError> {
        check_limits(limits)?;
        if !cfg!(any(unix, windows)) {
            return Err(RecoveryError::Unsupported);
        }
        if locator.scope != scope.key() {
            return Err(RecoveryError::ScopeMismatch);
        }
        let root = canonical_root(self, scope)?;
        let path = root.join(locator.name()?);
        check_owned_child(&path, &root).map_err(map_range)?;
        let (marker, attempt) = open_marker(&path)?;
        if attempt != locator.attempt {
            return Err(RecoveryError::OwnershipMarker);
        }
        fn data(path: &Path, name: &str, ceiling: u64) -> Result<File, RecoveryError> {
            let file = open(path.join(name).as_path())?;
            if file.metadata()?.len() > ceiling {
                return Err(RecoveryError::Bounds);
            }
            Ok(file)
        }
        let payload = data(&path, "payload", limits.payload_bytes)?;
        let proof = data(&path, "proof", limits.proof_bytes)?;
        let checkpoint = match data(&path, "checkpoint", limits.checkpoint_bytes) {
            Ok(f) => Some(f),
            Err(RecoveryError::Unavailable) => None,
            Err(e) => return Err(e),
        };
        let encoded = root.as_os_str().as_encoded_bytes();
        let mut canonical = [0; MAX_ROOT_BYTES];
        canonical[..encoded.len()].copy_from_slice(encoded);
        Ok(RangeRecoverySource {
            files: [Some(payload), Some(proof), checkpoint],
            _marker: marker,
            _root: scope.root(),
            canonical,
            root_len: encoded.len() as u16,
            scope: scope.key(),
            snapshot: scope.snapshot_id(),
            attempt,
            limits,
        })
    }
}
fn open(path: &Path) -> Result<File, RecoveryError> {
    let file = open_regular_exclusive(path).map_err(|e| match e {
        crate::sensing::local_custody::CustodyError::Io(e)
            if e.kind() == std::io::ErrorKind::NotFound =>
        {
            RecoveryError::Unavailable
        }
        crate::sensing::local_custody::CustodyError::Io(e) => RecoveryError::Io(e.kind()),
        _ => RecoveryError::NotRegular,
    })?;
    file.try_lock().map_err(|_| RecoveryError::Busy)?;
    Ok(file)
}
pub(super) fn open_marker(path: &Path) -> Result<(File, [u8; 32]), RecoveryError> {
    let mut file = open(&path.join(".owner"))?;
    if file.metadata()?.len() != 64 {
        return Err(RecoveryError::OwnershipMarker);
    }
    let mut bytes = [0; 64];
    file.read_exact(&mut bytes)?;
    let mut trailing = [0; 1];
    if file.read(&mut trailing)? != 0 {
        return Err(RecoveryError::OwnershipMarker);
    }
    let attempt =
        <[u8; 32]>::try_from(&bytes[16..48]).map_err(|_| RecoveryError::OwnershipMarker)?;
    if attempt == [0; 32] || bytes != owner_bytes(attempt) {
        return Err(RecoveryError::OwnershipMarker);
    }
    Ok((file, attempt))
}
impl<'root> RangeRecoverySource<'root> {
    pub const fn limits(&self) -> RangeLimits {
        self.limits
    }
    pub const fn attempt_id(&self) -> [u8; 32] {
        self.attempt
    }
    pub const fn snapshot_id(&self) -> [u8; 32] {
        self.snapshot
    }
    pub(crate) const fn scope_key(&self) -> RecoveryScopeKey {
        self.scope
    }
    pub fn matches_canonical_root(&self, path: &Path) -> bool {
        path.as_os_str().as_encoded_bytes() == &self.canonical[..usize::from(self.root_len)]
    }
    pub fn checkpoint_available(&self) -> bool {
        self.files[2].is_some()
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
    pub fn read_exact_at(
        &mut self,
        file: RangeFile,
        offset: u64,
        out: &mut [u8],
    ) -> Result<(), RecoveryError> {
        self.limits
            .check(file, offset, out.len())
            .map_err(|e| match e {
                crate::storage_driver::ranges::RangeError::Overflow => RecoveryError::Overflow,
                _ => RecoveryError::Bounds,
            })?;
        let handle = self.files[file.index()]
            .as_mut()
            .ok_or(RecoveryError::Unavailable)?;
        if out.is_empty() {
            return Ok(());
        }
        handle.seek(SeekFrom::Start(offset))?;
        handle.read_exact(out)?;
        Ok(())
    }
    pub(crate) fn read_view(&mut self) -> RangeRecoveryRead<'_, 'root> {
        RangeRecoveryRead { source: self }
    }
    /// Release actual handles only. Namespace and all old quota remain unchanged.
    pub fn close(self) {
        drop(self)
    }
}
/// Restricted borrow cannot replace/move the underlying owned source.
pub struct RangeRecoveryRead<'a, 'root> {
    source: &'a mut RangeRecoverySource<'root>,
}
impl RangeRecoveryRead<'_, '_> {
    pub fn limits(&self) -> RangeLimits {
        self.source.limits()
    }
    pub fn attempt_id(&self) -> [u8; 32] {
        self.source.attempt_id()
    }
    pub fn checkpoint_available(&self) -> bool {
        self.source.checkpoint_available()
    }
    pub fn matches_canonical_root(&self, path: &Path) -> bool {
        self.source.matches_canonical_root(path)
    }
    pub fn read_exact_at(
        &mut self,
        file: RangeFile,
        offset: u64,
        out: &mut [u8],
    ) -> Result<(), RecoveryError> {
        self.source.read_exact_at(file, offset, out)
    }
}
