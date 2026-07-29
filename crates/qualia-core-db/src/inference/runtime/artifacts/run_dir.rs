use std::fs;
use std::path::{Path, PathBuf};

use tempfile::{Builder, TempDir};

use super::budget::{validate_label, validate_relative_artifact_path, ArtifactError};

pub const RUN_MARKER_FILE: &str = ".qualia-inference-run";

#[derive(Debug, Clone)]
pub enum ArtifactRetention {
    Ephemeral,
    RetainTo(PathBuf),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArtifactStats {
    pub created_bytes: u64,
    pub removed_bytes: u64,
    pub retained_bytes: u64,
    pub cleanup_failures: u64,
}

#[derive(Debug)]
pub struct ArtifactFinish {
    pub retained_path: Option<PathBuf>,
    pub stats: ArtifactStats,
}

/// One bounded, marker-owned scratch directory.
///
/// Dropping an unfinished run removes it through [`TempDir`]. Call [`Self::finish`] to obtain
/// cleanup counters or atomically promote retained evidence.
#[derive(Debug)]
pub struct RunArtifactDir {
    dir: Option<TempDir>,
    retention: ArtifactRetention,
    byte_budget: u64,
    stats: ArtifactStats,
}

impl RunArtifactDir {
    pub fn new_in(
        scratch_parent: &Path,
        label: &str,
        byte_budget: u64,
        retention: ArtifactRetention,
    ) -> Result<Self, ArtifactError> {
        validate_label(label)?;
        fs::create_dir_all(scratch_parent)?;
        let dir = Builder::new()
            .prefix(&format!("qualia-inference-{label}-"))
            .tempdir_in(scratch_parent)?;
        fs::write(
            dir.path().join(RUN_MARKER_FILE),
            b"qualia-inference-run-v1\n",
        )?;
        Ok(Self {
            dir: Some(dir),
            retention,
            byte_budget,
            stats: ArtifactStats::default(),
        })
    }

    pub fn path(&self) -> &Path {
        self.dir
            .as_ref()
            .expect("artifact directory is unavailable after finish")
            .path()
    }

    pub fn remaining_bytes(&self) -> u64 {
        self.byte_budget.saturating_sub(self.stats.created_bytes)
    }

    pub fn stats(&self) -> ArtifactStats {
        self.stats
    }

    pub fn write_bounded(
        &mut self,
        relative_path: impl AsRef<Path>,
        bytes: &[u8],
    ) -> Result<PathBuf, ArtifactError> {
        let relative_path = relative_path.as_ref();
        validate_relative_artifact_path(relative_path)?;
        let attempted_bytes = self.stats.created_bytes.saturating_add(bytes.len() as u64);
        if attempted_bytes > self.byte_budget {
            return Err(ArtifactError::BudgetExceeded {
                budget_bytes: self.byte_budget,
                attempted_bytes,
            });
        }
        let output = self.path().join(relative_path);
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&output, bytes)?;
        self.stats.created_bytes = attempted_bytes;
        Ok(output)
    }

    pub fn finish(mut self) -> Result<ArtifactFinish, ArtifactError> {
        let dir = self
            .dir
            .take()
            .expect("artifact directory can only be finished once");
        match self.retention {
            ArtifactRetention::Ephemeral => {
                let path = dir.path().to_path_buf();
                match dir.close() {
                    Ok(()) => {
                        self.stats.removed_bytes = self.stats.created_bytes;
                        Ok(ArtifactFinish {
                            retained_path: None,
                            stats: self.stats,
                        })
                    }
                    Err(source) => {
                        self.stats.cleanup_failures = self.stats.cleanup_failures.saturating_add(1);
                        Err(ArtifactError::Cleanup { path, source })
                    }
                }
            }
            ArtifactRetention::RetainTo(target) => {
                if target.exists() {
                    return Err(ArtifactError::TargetExists);
                }
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                let staging = dir.keep();
                if let Err(source) = fs::rename(&staging, &target) {
                    return Err(ArtifactError::Promotion {
                        staging,
                        target,
                        source,
                    });
                }
                self.stats.retained_bytes = self.stats.created_bytes;
                Ok(ArtifactFinish {
                    retained_path: Some(target),
                    stats: self.stats,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    use super::*;

    #[test]
    fn ephemeral_finish_removes_run_directory() {
        let parent = tempfile::tempdir().unwrap();
        let mut run =
            RunArtifactDir::new_in(parent.path(), "cleanup", 16, ArtifactRetention::Ephemeral)
                .unwrap();
        let run_path = run.path().to_path_buf();
        run.write_bounded("nested/out.bin", b"1234").unwrap();
        let finish = run.finish().unwrap();
        assert!(!run_path.exists());
        assert_eq!(finish.stats.created_bytes, 4);
        assert_eq!(finish.stats.removed_bytes, 4);
        assert_eq!(finish.stats.retained_bytes, 0);
    }

    #[test]
    fn drop_removes_directory_during_unwind() {
        let parent = tempfile::tempdir().unwrap();
        let mut run_path = PathBuf::new();
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut run =
                RunArtifactDir::new_in(parent.path(), "panic", 16, ArtifactRetention::Ephemeral)
                    .unwrap();
            run_path = run.path().to_path_buf();
            run.write_bounded("out.bin", b"1234").unwrap();
            panic!("test unwind");
        }));
        assert!(result.is_err());
        assert!(!run_path.exists());
    }

    #[test]
    fn budget_fails_closed_without_partial_file() {
        let parent = tempfile::tempdir().unwrap();
        let mut run =
            RunArtifactDir::new_in(parent.path(), "budget", 3, ArtifactRetention::Ephemeral)
                .unwrap();
        let err = run.write_bounded("too-large.bin", b"1234").unwrap_err();
        assert!(matches!(err, ArtifactError::BudgetExceeded { .. }));
        assert!(!run.path().join("too-large.bin").exists());
    }

    #[test]
    fn traversal_is_rejected() {
        let parent = tempfile::tempdir().unwrap();
        let mut run =
            RunArtifactDir::new_in(parent.path(), "paths", 16, ArtifactRetention::Ephemeral)
                .unwrap();
        assert!(matches!(
            run.write_bounded("../escape.bin", b"x"),
            Err(ArtifactError::InvalidRelativePath)
        ));
    }

    #[test]
    fn retained_run_is_promoted_out_of_staging() {
        let parent = tempfile::tempdir().unwrap();
        let target = parent.path().join("evidence").join("run-1");
        let mut run = RunArtifactDir::new_in(
            parent.path(),
            "retain",
            16,
            ArtifactRetention::RetainTo(target.clone()),
        )
        .unwrap();
        let staging = run.path().to_path_buf();
        run.write_bounded("receipt.json", b"{}").unwrap();
        let finish = run.finish().unwrap();
        assert!(!staging.exists());
        assert_eq!(finish.retained_path.as_deref(), Some(target.as_path()));
        assert!(target.join(RUN_MARKER_FILE).is_file());
        assert_eq!(finish.stats.retained_bytes, 2);
    }
}
