//! Bounded import job: unique TempDir, streaming chunks, promote-on-success.

use super::budgets::ImportBudgets;
use super::error::ImportError;
use super::status::{ChunkOutcome, FeedProgress, ImportStatus, PromotedArtifact};
use crate::q42::asset_envelope::sha256_of;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const RAW_NAME: &str = "raw.bin";
const OUT_NAME: &str = "accepted.bin";
const PROMOTE_NAME: &str = "promoted.bin";

/// Cold-construction import job with RAII scratch cleanup.
///
/// The caller's `raw_path` is never opened for write. A copy (or same-volume
/// hardlink) lives under a unique [`TempDir`]; only that directory is cleaned
/// on drop / unwind — no broad temp sweeps.
pub struct ImportJob {
    scratch: TempDir,
    raw_scratch: PathBuf,
    out_scratch: PathBuf,
    reader: File,
    budgets: ImportBudgets,
    cancelled: bool,
    bytes_read: u64,
    accepted: u64,
    quarantined: u64,
    output_bytes: u64,
    status: ImportStatus,
    /// Scratch root path retained for cleanup tests after `into_parts` / drop.
    scratch_path: PathBuf,
}

impl ImportJob {
    /// Begin a job: validate budgets, create unique TempDir, stage immutable raw.
    pub fn begin(raw_path: &Path, budgets: ImportBudgets) -> Result<Self, ImportError> {
        let budgets = budgets.validate()?;
        if !raw_path.is_file() {
            return Err(ImportError::RawNotFound(raw_path.to_path_buf()));
        }

        let scratch = TempDir::new().map_err(ImportError::Io)?;
        let scratch_path = scratch.path().to_path_buf();
        let raw_scratch = scratch_path.join(RAW_NAME);
        let out_scratch = scratch_path.join(OUT_NAME);

        stage_immutable_raw(raw_path, &raw_scratch)?;
        File::create(&out_scratch).map_err(ImportError::Io)?;

        let reader = File::open(&raw_scratch).map_err(ImportError::Io)?;

        Ok(Self {
            scratch,
            raw_scratch,
            out_scratch,
            reader,
            budgets,
            cancelled: false,
            bytes_read: 0,
            accepted: 0,
            quarantined: 0,
            output_bytes: 0,
            status: ImportStatus::Running,
            scratch_path,
        })
    }

    /// Absolute path of this job's TempDir (for tests / diagnostics only).
    pub fn scratch_path(&self) -> &Path {
        &self.scratch_path
    }

    /// Path of the immutable staged raw copy inside scratch.
    pub fn raw_scratch_path(&self) -> &Path {
        &self.raw_scratch
    }

    pub fn status(&self) -> &ImportStatus {
        &self.status
    }

    pub fn accepted(&self) -> u64 {
        self.accepted
    }

    pub fn quarantined(&self) -> u64 {
        self.quarantined
    }

    pub fn budgets(&self) -> ImportBudgets {
        self.budgets
    }

    /// Request cancellation; subsequent `feed_chunk` / `promote` fail closed.
    pub fn cancel(&mut self) {
        self.cancelled = true;
        if matches!(self.status, ImportStatus::Running) {
            self.status = ImportStatus::Cancelled;
        }
    }

    /// Stream the next chunk under `chunk_byte_budget`, invoke `f`, update counts.
    ///
    /// `f` receives the chunk bytes and a writer for accepted output bytes.
    /// On EOF after a successful callback the job transitions to [`ImportStatus::Succeeded`].
    pub fn feed_chunk<F>(&mut self, f: F) -> Result<FeedProgress, ImportError>
    where
        F: FnOnce(&[u8], &mut dyn Write) -> Result<ChunkOutcome, ImportError>,
    {
        if self.cancelled || matches!(self.status, ImportStatus::Cancelled) {
            self.status = ImportStatus::Cancelled;
            return Err(ImportError::Cancelled);
        }
        if let ImportStatus::Failed(msg) = &self.status {
            return Err(ImportError::Failed(msg.clone()));
        }
        if self.status.is_succeeded() {
            return Ok(FeedProgress {
                bytes_this_chunk: 0,
                bytes_read_total: self.bytes_read,
                eof: true,
            });
        }

        let remaining_bytes = self.budgets.max_bytes.saturating_sub(self.bytes_read);
        if remaining_bytes == 0 {
            return self.fail_budget_bytes();
        }

        let to_read = (self.budgets.chunk_byte_budget as usize).min(remaining_bytes as usize);

        let mut buf = vec![0u8; to_read];
        let n = self.reader.read(&mut buf).map_err(|e| {
            self.status = ImportStatus::Failed(e.to_string());
            ImportError::Io(e)
        })?;
        buf.truncate(n);

        if n == 0 {
            self.status = ImportStatus::Succeeded {
                bytes_read: self.bytes_read,
                accepted: self.accepted,
                quarantined: self.quarantined,
                output_bytes: self.output_bytes,
            };
            return Ok(FeedProgress {
                bytes_this_chunk: 0,
                bytes_read_total: self.bytes_read,
                eof: true,
            });
        }

        // Hard fail if this chunk alone would exceed max_bytes (partial last chunk OK).
        let next_total = self.bytes_read.saturating_add(n as u64);
        if next_total > self.budgets.max_bytes {
            return self.fail_budget_bytes();
        }

        let mut out = File::options()
            .append(true)
            .open(&self.out_scratch)
            .map_err(|e| {
                self.status = ImportStatus::Failed(e.to_string());
                ImportError::Io(e)
            })?;

        let before_len = self.output_bytes;
        let outcome = match f(&buf, &mut out) {
            Ok(o) => o,
            Err(e) => {
                self.status = ImportStatus::Failed(e.to_string());
                return Err(e);
            }
        };
        out.flush().map_err(|e| {
            self.status = ImportStatus::Failed(e.to_string());
            ImportError::Io(e)
        })?;
        drop(out);

        let written = fs::metadata(&self.out_scratch)
            .map(|m| m.len())
            .unwrap_or(before_len);
        self.output_bytes = written;

        let added = outcome.accepted.saturating_add(outcome.quarantined);
        let next_records = self.accepted.saturating_add(self.quarantined).saturating_add(added);
        if next_records > self.budgets.max_records {
            self.status = ImportStatus::Failed("record budget exceeded".into());
            return Err(ImportError::RecordBudgetExceeded {
                used: next_records,
                max: self.budgets.max_records,
            });
        }

        self.bytes_read = next_total;
        self.accepted = self.accepted.saturating_add(outcome.accepted);
        self.quarantined = self.quarantined.saturating_add(outcome.quarantined);

        // Peek one byte to detect EOF without consuming a future chunk budget incorrectly.
        let mut peek = [0u8; 1];
        let eof = match self.reader.read(&mut peek) {
            Ok(0) => true,
            Ok(1) => {
                // Put the byte back by seeking -1; File is Seek for regular files.
                use std::io::Seek;
                self.reader
                    .seek(io::SeekFrom::Current(-1))
                    .map_err(|e| {
                        self.status = ImportStatus::Failed(e.to_string());
                        ImportError::Io(e)
                    })?;
                false
            }
            Ok(_) => false,
            Err(e) => {
                self.status = ImportStatus::Failed(e.to_string());
                return Err(ImportError::Io(e));
            }
        };

        // If we hit max_bytes exactly mid-stream without EOF, fail closed on next feed;
        // if at max_bytes and more data remains, that is a budget violation.
        if !eof && self.bytes_read >= self.budgets.max_bytes {
            return self.fail_budget_bytes();
        }

        if eof {
            self.status = ImportStatus::Succeeded {
                bytes_read: self.bytes_read,
                accepted: self.accepted,
                quarantined: self.quarantined,
                output_bytes: self.output_bytes,
            };
        }

        Ok(FeedProgress {
            bytes_this_chunk: n as u64,
            bytes_read_total: self.bytes_read,
            eof,
        })
    }

    /// Promote staged accepted output into `dest_dir` (only when status is Succeeded).
    ///
    /// Consumes `self` so the TempDir drops after the promote copy completes.
    pub fn promote(self, dest_dir: &Path) -> Result<PromotedArtifact, ImportError> {
        if self.cancelled || matches!(self.status, ImportStatus::Cancelled) {
            return Err(ImportError::Cancelled);
        }
        let (_bytes_read, accepted, quarantined, output_bytes) =
            self.status.require_succeeded()?;

        fs::create_dir_all(dest_dir).map_err(ImportError::Io)?;
        let dest = dest_dir.join(PROMOTE_NAME);
        if dest.exists() {
            return Err(ImportError::DestinationExists(dest));
        }

        fs::copy(&self.out_scratch, &dest).map_err(ImportError::Io)?;
        let bytes = fs::metadata(&dest).map(|m| m.len()).unwrap_or(output_bytes);
        let data = fs::read(&dest).map_err(ImportError::Io)?;
        let digest = sha256_of(&data);

        // Drop scratch via self.scratch Drop at end of scope.
        let _scratch = self.scratch;

        Ok(PromotedArtifact {
            path: dest,
            sha256: digest,
            bytes,
            accepted,
            quarantined,
        })
    }

    fn fail_budget_bytes(&mut self) -> Result<FeedProgress, ImportError> {
        self.status = ImportStatus::Failed("byte budget exceeded".into());
        Err(ImportError::ByteBudgetExceeded {
            used: self.bytes_read,
            max: self.budgets.max_bytes,
        })
    }
}

/// Copy or hardlink `src` into `dst` without mutating `src`.
fn stage_immutable_raw(src: &Path, dst: &Path) -> Result<(), ImportError> {
    // Prefer hardlink when same volume (zero-copy stage); fall back to full copy.
    match fs::hard_link(src, dst) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(src, dst).map_err(ImportError::Io)?;
            Ok(())
        }
    }
}
