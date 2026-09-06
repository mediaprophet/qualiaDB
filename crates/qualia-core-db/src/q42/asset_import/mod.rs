//! Bounded Q42 asset import jobs (AST-02).
//!
//! Cold-construction framework: unique [`tempfile::TempDir`], explicit byte /
//! record / chunk budgets (chunk pass ≤ [`SENTINEL_PASS_BUDGET_BYTES`]), streaming
//! `Read`, cancellation, quarantine counts, and promote-on-success. Caller
//! supplies a local [`Path`] — no network downloader. The raw caller path is
//! never mutated; only the job TempDir is cleaned via RAII.

mod budgets;
mod error;
mod job;
mod status;

pub use budgets::ImportBudgets;
pub use error::ImportError;
pub use job::ImportJob;
pub use status::{ChunkOutcome, FeedProgress, ImportStatus, PromotedArtifact};

pub use crate::q42::asset_envelope::SENTINEL_PASS_BUDGET_BYTES;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42::asset_envelope::sha256_of;
    use std::fs;
    use std::path::PathBuf;

    fn write_raw(dir: &tempfile::TempDir, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, bytes).unwrap();
        path
    }

    fn small_budgets(max_bytes: u64, max_records: u64, chunk: u64) -> ImportBudgets {
        ImportBudgets {
            max_bytes,
            max_records,
            chunk_byte_budget: chunk,
        }
    }

    fn feed_passthrough(job: &mut ImportJob) -> Result<FeedProgress, ImportError> {
        job.feed_chunk(|chunk, out| {
            out.write_all(chunk).map_err(ImportError::Io)?;
            Ok(ChunkOutcome {
                accepted: 1,
                quarantined: 0,
            })
        })
    }

    fn drain_to_success(job: &mut ImportJob) -> Result<(), ImportError> {
        loop {
            let progress = feed_passthrough(job)?;
            if progress.eof {
                return Ok(());
            }
        }
    }

    #[test]
    fn chunk_over_sentinel_rejected_at_begin() {
        let dir = tempfile::TempDir::new().unwrap();
        let raw = write_raw(&dir, "a.bin", b"abc");
        let budgets = ImportBudgets {
            max_bytes: SENTINEL_PASS_BUDGET_BYTES,
            max_records: 10,
            chunk_byte_budget: SENTINEL_PASS_BUDGET_BYTES + 1,
        };
        assert_eq!(
            ImportJob::begin(&raw, budgets).err(),
            Some(ImportError::ChunkBudgetExceeded)
        );
    }

    #[test]
    fn budget_exceeded_when_raw_larger_than_max_bytes() {
        let dir = tempfile::TempDir::new().unwrap();
        let payload = b"0123456789ABCDEF"; // 16 bytes
        let raw = write_raw(&dir, "big.bin", payload);
        let mut job = ImportJob::begin(&raw, small_budgets(8, 100, 4)).unwrap();
        // First chunk: 4 bytes OK
        let p1 = feed_passthrough(&mut job).unwrap();
        assert_eq!(p1.bytes_this_chunk, 4);
        assert!(!p1.eof);
        // Second chunk: 4 bytes → at max_bytes with more remaining → fail
        let err = feed_passthrough(&mut job).unwrap_err();
        assert_eq!(
            err,
            ImportError::ByteBudgetExceeded {
                used: 8,
                max: 8
            }
        );
        assert!(matches!(job.status(), ImportStatus::Failed(_)));
    }

    #[test]
    fn cleanup_on_success() {
        let dir = tempfile::TempDir::new().unwrap();
        let payload = b"promote-me";
        let raw = write_raw(&dir, "in.bin", payload);
        let dest = dir.path().join("out");
        let scratch_path = {
            let mut job = ImportJob::begin(&raw, small_budgets(64, 10, 32)).unwrap();
            let scratch = job.scratch_path().to_path_buf();
            assert!(scratch.exists());
            drain_to_success(&mut job).unwrap();
            assert!(job.status().is_succeeded());
            let promoted = job.promote(&dest).unwrap();
            assert_eq!(promoted.bytes, payload.len() as u64);
            assert_eq!(promoted.sha256, sha256_of(payload));
            assert!(promoted.path.exists());
            scratch
        };
        assert!(
            !scratch_path.exists(),
            "TempDir must be gone after successful promote"
        );
        // Caller raw path unchanged and still present.
        assert_eq!(fs::read(&raw).unwrap(), payload);
    }

    #[test]
    fn cleanup_on_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let raw = write_raw(&dir, "in.bin", b"data");
        let scratch_path = {
            let mut job = ImportJob::begin(&raw, small_budgets(64, 10, 32)).unwrap();
            let scratch = job.scratch_path().to_path_buf();
            let err = job.feed_chunk(|_, _| Err(ImportError::Failed("parse".into())));
            assert_eq!(err, Err(ImportError::Failed("parse".into())));
            assert!(scratch.exists(), "scratch lives while job is held");
            scratch
        };
        assert!(
            !scratch_path.exists(),
            "TempDir RAII cleanup on drop after error"
        );
    }

    #[test]
    fn cleanup_on_unwind() {
        let dir = tempfile::TempDir::new().unwrap();
        let raw = write_raw(&dir, "in.bin", b"unwind-payload");
        let scratch_gone = {
            let job = ImportJob::begin(&raw, small_budgets(64, 10, 32)).unwrap();
            let scratch = job.scratch_path().to_path_buf();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut job = job;
                let _ = job.feed_chunk(|_, _| panic!("forced unwind"));
            }));
            assert!(result.is_err());
            scratch
        };
        assert!(
            !scratch_gone.exists(),
            "TempDir must clean up on unwind"
        );
    }

    #[test]
    fn cancellation_blocks_feed_and_promote() {
        let dir = tempfile::TempDir::new().unwrap();
        let raw = write_raw(&dir, "in.bin", b"0123456789");
        let mut job = ImportJob::begin(&raw, small_budgets(64, 10, 4)).unwrap();
        let _ = feed_passthrough(&mut job).unwrap();
        job.cancel();
        assert_eq!(job.status(), &ImportStatus::Cancelled);
        assert_eq!(feed_passthrough(&mut job).err(), Some(ImportError::Cancelled));
        let dest = dir.path().join("cancelled-out");
        assert_eq!(job.promote(&dest).err(), Some(ImportError::Cancelled));
    }

    #[test]
    fn partial_input_leaves_running_and_cleans_on_drop() {
        let dir = tempfile::TempDir::new().unwrap();
        let raw = write_raw(&dir, "in.bin", b"abcdefghij"); // 10 bytes
        let dest = dir.path().join("partial-out");

        // Promote with no successful feed must fail closed.
        let early = ImportJob::begin(&raw, small_budgets(64, 10, 4)).unwrap();
        assert_eq!(early.promote(&dest).err(), Some(ImportError::NotSucceeded));

        let scratch_path = {
            let mut job = ImportJob::begin(&raw, small_budgets(64, 10, 4)).unwrap();
            let scratch = job.scratch_path().to_path_buf();
            let p = feed_passthrough(&mut job).unwrap();
            assert_eq!(p.bytes_this_chunk, 4);
            assert!(!p.eof);
            assert_eq!(job.status(), &ImportStatus::Running);
            // Drop without promote — RAII cleans scratch.
            scratch
        };
        assert!(!scratch_path.exists());
    }

    #[test]
    fn deterministic_output_hash_of_promoted_bytes() {
        let dir = tempfile::TempDir::new().unwrap();
        let payload = b"hash-stable-fixture-v1";
        let raw = write_raw(&dir, "in.bin", payload);
        let dest_a = dir.path().join("a");
        let dest_b = dir.path().join("b");

        let hash_a = {
            let mut job = ImportJob::begin(&raw, ImportBudgets::sentinel_pass()).unwrap();
            drain_to_success(&mut job).unwrap();
            job.promote(&dest_a).unwrap().sha256
        };
        let hash_b = {
            let mut job = ImportJob::begin(&raw, ImportBudgets::sentinel_pass()).unwrap();
            drain_to_success(&mut job).unwrap();
            job.promote(&dest_b).unwrap().sha256
        };
        assert_eq!(hash_a, hash_b);
        assert_eq!(hash_a, sha256_of(payload));
    }

    #[test]
    fn raw_caller_path_not_mutated() {
        let dir = tempfile::TempDir::new().unwrap();
        let payload = b"immutable-raw";
        let raw = write_raw(&dir, "caller.bin", payload);
        let before = fs::metadata(&raw).unwrap().len();
        let mut job = ImportJob::begin(&raw, small_budgets(64, 10, 32)).unwrap();
        // Scratch raw must differ in path from caller.
        assert_ne!(job.raw_scratch_path(), raw.as_path());
        drain_to_success(&mut job).unwrap();
        let _ = job.promote(&dir.path().join("promoted-dir")).unwrap();
        assert_eq!(fs::read(&raw).unwrap(), payload);
        assert_eq!(fs::metadata(&raw).unwrap().len(), before);
    }

    #[test]
    fn quarantine_counts_accumulate() {
        let dir = tempfile::TempDir::new().unwrap();
        let raw = write_raw(&dir, "in.bin", b"ABCDEFGH");
        let mut job = ImportJob::begin(&raw, small_budgets(64, 100, 4)).unwrap();
        job.feed_chunk(|chunk, out| {
            out.write_all(&chunk[..2]).map_err(ImportError::Io)?;
            Ok(ChunkOutcome {
                accepted: 2,
                quarantined: 2,
            })
        })
        .unwrap();
        assert_eq!(job.accepted(), 2);
        assert_eq!(job.quarantined(), 2);
    }

    #[test]
    fn record_budget_exceeded() {
        let dir = tempfile::TempDir::new().unwrap();
        let raw = write_raw(&dir, "in.bin", b"ABCDEFGH");
        let mut job = ImportJob::begin(&raw, small_budgets(64, 3, 8)).unwrap();
        let err = job
            .feed_chunk(|chunk, out| {
                out.write_all(chunk).map_err(ImportError::Io)?;
                Ok(ChunkOutcome {
                    accepted: 2,
                    quarantined: 2,
                })
            })
            .unwrap_err();
        assert_eq!(
            err,
            ImportError::RecordBudgetExceeded { used: 4, max: 3 }
        );
    }
}
