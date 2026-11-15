use super::super::session::{check_owned_child, checked_root, is_reparse};
use super::source::open_marker;
use super::types::*;
use crate::{
    sensing::local_custody::{quota::recovery::RecoveryRootScope, validate_root},
    storage_driver::MmapDriver,
};
use std::fs;
impl MmapDriver {
    /// Cold bounded local discovery only. Counters bound high-level filesystem
    /// steps (bounded helpers), not exact OS-internal syscalls. No incoming path is accepted.
    pub fn lookup_range_sources(
        &self,
        scope: &RecoveryRootScope<'_>,
        attempt: [u8; 32],
        limits: LookupLimits,
        out: &mut [Option<StagingLocator>],
    ) -> Result<LookupReport, RecoveryError> {
        if out.len() > 8 {
            return Err(RecoveryError::InvalidLimits);
        }
        out.iter_mut().for_each(|slot| *slot = None);
        let result = lookup(self, scope, attempt, limits, out);
        if result.is_err() {
            out.iter_mut().for_each(|slot| *slot = None)
        }
        result
    }
}
pub(super) fn canonical_root(
    driver: &MmapDriver,
    scope: &RecoveryRootScope<'_>,
) -> Result<std::path::PathBuf, RecoveryError> {
    validate_root(&driver.root).map_err(|_| RecoveryError::InvalidRoot)?;
    validate_root(scope.root()).map_err(|_| RecoveryError::InvalidRoot)?;
    let actual = checked_root(&driver.root).map_err(map_range)?;
    let expected = checked_root(scope.root()).map_err(map_range)?;
    if actual != expected || actual.as_os_str().as_encoded_bytes().len() > MAX_ROOT_BYTES {
        return Err(RecoveryError::ScopeMismatch);
    }
    Ok(actual)
}
pub(super) fn map_range(e: crate::storage_driver::ranges::RangeError) -> RecoveryError {
    match e {
        crate::storage_driver::ranges::RangeError::Io(kind) => RecoveryError::Io(kind),
        _ => RecoveryError::InvalidRoot,
    }
}
fn tick(used: &mut u32, limit: u32) -> Result<(), RecoveryError> {
    *used = used.checked_add(1).ok_or(RecoveryError::Overflow)?;
    if *used > limit {
        return Err(RecoveryError::Incomplete);
    }
    Ok(())
}
fn lookup(
    driver: &MmapDriver,
    scope: &RecoveryRootScope<'_>,
    attempt: [u8; 32],
    limits: LookupLimits,
    out: &mut [Option<StagingLocator>],
) -> Result<LookupReport, RecoveryError> {
    if attempt == [0; 32] {
        return Err(RecoveryError::InvalidAttempt);
    }
    if out.is_empty() || limits.entries == 0 || limits.io_steps < 8 {
        return Err(RecoveryError::InvalidLimits);
    }
    let mut calls = 0;
    tick(&mut calls, limits.io_steps)?;
    let root = canonical_root(driver, scope)?;
    tick(&mut calls, limits.io_steps)?;
    let mut entries = fs::read_dir(&root)?;
    let mut scanned = 0u32;
    let mut found = 0usize;
    loop {
        tick(&mut calls, limits.io_steps)?;
        let Some(entry) = entries.next() else { break };
        if scanned == limits.entries {
            return Err(RecoveryError::Incomplete);
        }
        scanned += 1;
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.len() > MAX_RUN_NAME_BYTES
            || !name.starts_with("qualia-range-")
            || name.len() <= 13
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            continue;
        }
        let path = entry.path();
        tick(&mut calls, limits.io_steps)?;
        let meta = fs::symlink_metadata(&path)?;
        if !meta.is_dir() || meta.file_type().is_symlink() || is_reparse(&meta) {
            continue;
        }
        tick(&mut calls, limits.io_steps)?;
        if check_owned_child(&path, &root).is_err() {
            continue;
        }
        tick(&mut calls, limits.io_steps)?;
        let (_, seen) = match open_marker(&path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if seen != attempt {
            continue;
        }
        if found != 0 {
            return Err(RecoveryError::Ambiguous);
        }
        let mut encoded = [0; MAX_RUN_NAME_BYTES];
        encoded[..name.len()].copy_from_slice(name.as_bytes());
        out[found] = Some(StagingLocator {
            name: encoded,
            len: name.len() as u8,
            attempt,
            scope: scope.key(),
        });
        found += 1;
    }
    if found == 0 {
        return Err(RecoveryError::Unavailable);
    }
    Ok(LookupReport {
        entries: scanned,
        io_steps: calls,
        matches: found,
    })
}
