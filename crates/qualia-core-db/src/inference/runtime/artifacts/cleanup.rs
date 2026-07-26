use std::fs;
use std::path::Path;
use std::time::SystemTime;

use super::budget::ArtifactError;
use super::run_dir::RUN_MARKER_FILE;

const RUN_PREFIX: &str = "qualia-inference-";
const MARKER_CONTENT: &[u8] = b"qualia-inference-run-v1\n";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StaleCleanupReport {
    pub candidates: u64,
    pub removed_runs: u64,
    pub removed_bytes: u64,
    pub failures: u64,
}

fn directory_bytes(path: &Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            total = total.saturating_add(directory_bytes(&entry.path())?);
        } else if file_type.is_file() {
            total = total.saturating_add(entry.metadata()?.len());
        }
    }
    Ok(total)
}

/// Remove only stale, marker-owned staging runs that are direct children of `parent`.
///
/// Retained evidence has a caller-chosen name and is never eligible. Symlink candidates are
/// rejected, canonical parent containment is verified, and individual failures are counted.
pub fn cleanup_stale_runs(
    parent: &Path,
    modified_before: SystemTime,
) -> Result<StaleCleanupReport, ArtifactError> {
    let canonical_parent = parent.canonicalize()?;
    let mut report = StaleCleanupReport::default();
    for entry in fs::read_dir(&canonical_parent)? {
        let Ok(entry) = entry else {
            report.failures = report.failures.saturating_add(1);
            continue;
        };
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !name.starts_with(RUN_PREFIX) {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            report.failures = report.failures.saturating_add(1);
            continue;
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        report.candidates = report.candidates.saturating_add(1);
        let path = entry.path();
        let marker = path.join(RUN_MARKER_FILE);
        if fs::read(&marker).ok().as_deref() != Some(MARKER_CONTENT) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            report.failures = report.failures.saturating_add(1);
            continue;
        };
        if metadata
            .modified()
            .map_or(true, |time| time >= modified_before)
        {
            continue;
        }
        let Ok(canonical_path) = path.canonicalize() else {
            report.failures = report.failures.saturating_add(1);
            continue;
        };
        if canonical_path.parent() != Some(canonical_parent.as_path()) {
            report.failures = report.failures.saturating_add(1);
            continue;
        }
        let bytes = directory_bytes(&canonical_path).unwrap_or(0);
        match fs::remove_dir_all(&canonical_path) {
            Ok(()) => {
                report.removed_runs = report.removed_runs.saturating_add(1);
                report.removed_bytes = report.removed_bytes.saturating_add(bytes);
            }
            Err(_) => report.failures = report.failures.saturating_add(1),
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_only_marker_owned_prefixed_directories() {
        let parent = tempfile::tempdir().unwrap();
        let stale = parent.path().join("qualia-inference-test-stale");
        let unmarked = parent.path().join("qualia-inference-test-unmarked");
        let retained = parent.path().join("retained-evidence");
        fs::create_dir_all(&stale).unwrap();
        fs::create_dir_all(&unmarked).unwrap();
        fs::create_dir_all(&retained).unwrap();
        fs::write(stale.join(RUN_MARKER_FILE), MARKER_CONTENT).unwrap();
        fs::write(stale.join("payload.bin"), [1u8; 8]).unwrap();
        fs::write(retained.join(RUN_MARKER_FILE), MARKER_CONTENT).unwrap();

        let report = cleanup_stale_runs(
            parent.path(),
            SystemTime::now() + std::time::Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(report.removed_runs, 1);
        assert!(report.removed_bytes >= 8);
        assert!(!stale.exists());
        assert!(unmarked.exists());
        assert!(retained.exists());
    }
}
