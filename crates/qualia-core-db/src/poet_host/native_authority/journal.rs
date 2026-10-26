//! Bounded create-new journal. No serialized session can restore authority.
use super::admission::{BoundedBytes, SignedAdmission, TrustedProject};
use super::PackageWriteEvidence;
use qualia_cooperative_core::authoring::{ContributionRecord, DocumentRevision, ReviewRecord};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DurableState {
    pub admission: SignedAdmission,
    pub document: DocumentRevision,
    pub contributions: Vec<ContributionRecord>,
    pub reviews: Vec<ReviewRecord>,
    pub packages: Vec<PackageWriteEvidence>,
    #[serde(default)]
    pub requests: Vec<super::SourceReviewRequest>,
    #[serde(default)]
    pub enrollment: Vec<super::enrollment::EnrollmentRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub package_contributions: Vec<super::package_review::PackageContributionRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub package_requests: Vec<super::package_review::PackageReviewRequest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub package_reviews: Vec<super::package_review::PackageReviewRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub package_grant_decisions: Vec<super::package_review::PackageGrantDecision>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    sequence: u64,
    previous: [u8; 32],
    state: DurableState,
}

pub(super) struct Journal {
    root: PathBuf,
    configured_root: PathBuf,
    _ownership: fs::File,
    sequence: u64,
    digest: [u8; 32],
    max_bytes: usize,
    max_entries: u64,
}
impl Journal {
    pub fn preflight_package(&self, state: &DurableState) -> Result<(), String> {
        if self.sequence >= self.max_entries {
            return Err("authority journal entry budget exhausted before package write".into());
        }
        // One package audit record contains bounded document/principal text and
        // fixed digests. Reserve its worst-case escaped JSON before file IO.
        let reserve = 256 * 1024;
        let limit = self
            .max_bytes
            .checked_sub(reserve)
            .ok_or("authority entry has no package audit reserve")?;
        let mut writer = BoundedBytes {
            bytes: Vec::new(),
            limit,
        };
        serde_json::to_writer(
            &mut writer,
            &Entry {
                sequence: self.sequence + 1,
                previous: self.digest,
                state: state.clone(),
            },
        )
        .map_err(|e| format!("authority package audit byte preflight: {e}"))?;
        Ok(())
    }
    pub fn open(
        root: &Path,
        anchor: &TrustedProject,
        max_bytes: usize,
        max_entries: u64,
    ) -> Result<(Self, Option<DurableState>), String> {
        let meta =
            fs::symlink_metadata(root).map_err(|e| format!("authority journal directory: {e}"))?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err("authority journal requires a native host-owned non-link directory".into());
        }
        let canonical_root = root.canonicalize().map_err(|e| e.to_string())?;
        let ownership_path = canonical_root.join(".owner.lock");
        if ownership_path.exists() {
            let metadata = fs::symlink_metadata(&ownership_path).map_err(|e| e.to_string())?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err("native authority ownership sidecar is not a regular file".into());
            }
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            // Keep the stable ownership file from being renamed/deleted while
            // the native process holds it. Competing owners must take its lock.
            options.share_mode(3);
        }
        let ownership = options.open(&ownership_path).map_err(|e| e.to_string())?;
        ownership
            .try_lock()
            .map_err(|e| format!("authority journal already owned/unavailable: {e}"))?;
        let mut paths = Vec::new();
        for entry in fs::read_dir(&canonical_root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_name() == ".owner.lock" {
                continue;
            }
            if paths.len() as u64 >= max_entries {
                return Err("authority journal entry budget exceeded".into());
            }
            paths.push(entry.path());
        }
        paths.sort();
        let mut journal = Self {
            root: canonical_root,
            configured_root: root.into(),
            _ownership: ownership,
            sequence: 0,
            digest: [0; 32],
            max_bytes,
            max_entries,
        };
        let mut latest: Option<DurableState> = None;
        let mut last_admission_sequence = 0;
        let mut last_admission_digest = None;
        for path in paths {
            let next = journal
                .sequence
                .checked_add(1)
                .ok_or("journal sequence overflow")?;
            if path != journal.path(next) {
                return Err("authority journal has an unknown entry or sequence gap".into());
            }
            let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || metadata.len() > max_bytes as u64
            {
                return Err("authority journal entry violates file/byte bounds".into());
            }
            let mut bytes = Vec::new();
            fs::File::open(&path)
                .map_err(|e| e.to_string())?
                .take(max_bytes as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > max_bytes {
                return Err("authority journal entry grew beyond byte budget".into());
            }
            let entry: Entry = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if entry.sequence != next || entry.previous != journal.digest {
                return Err("authority journal chain mismatch".into());
            }
            let admission_digest = entry.state.admission.verify(anchor, max_bytes)?;
            if entry.state.admission.snapshot.sequence < last_admission_sequence
                || (entry.state.admission.snapshot.sequence == last_admission_sequence
                    && last_admission_digest != Some(admission_digest))
                || entry.state.document.project_id != anchor.project_id
                || entry.state.contributions.len() > 256
                || entry.state.reviews.len() > 256
                || entry.state.packages.len() > 256
                || entry.state.requests.len() > 256
                || entry.state.enrollment.len() > 256
            {
                return Err(
                    "authority journal regresses admission or exceeds evidence bounds".into(),
                );
            }
            super::package_review::validate_ledger(&entry.state)?;
            if let Some(previous) = latest.as_ref() {
                super::package_review::validate_grant_successor(
                    &previous.admission.snapshot,
                    &entry.state.admission.snapshot,
                )?;
            }
            last_admission_sequence = entry.state.admission.snapshot.sequence;
            last_admission_digest = Some(admission_digest);
            journal.sequence = next;
            journal.digest = Sha256::digest(&bytes).into();
            latest = Some(entry.state);
        }
        Ok((journal, latest))
    }
    fn path(&self, sequence: u64) -> PathBuf {
        self.root.join(format!("{sequence:020}.json"))
    }
    pub fn append(&mut self, state: &DurableState) -> Result<(), String> {
        super::package_review::validate_ledger(state)?;
        if self
            .configured_root
            .canonicalize()
            .map_err(|e| e.to_string())?
            != self.root
            || self.root.canonicalize().map_err(|e| e.to_string())? != self.root
        {
            return Err("native authority journal directory changed".into());
        }
        let next = self
            .sequence
            .checked_add(1)
            .filter(|n| *n <= self.max_entries)
            .ok_or("authority journal entry budget exhausted")?;
        let mut writer = BoundedBytes {
            bytes: Vec::new(),
            limit: self.max_bytes,
        };
        serde_json::to_writer(
            &mut writer,
            &Entry {
                sequence: next,
                previous: self.digest,
                state: state.clone(),
            },
        )
        .map_err(|e| e.to_string())?;
        // A failed/partial append is preserved and makes reopen fail closed.
        // The owner latches reconciliation rather than skipping/replacing it.
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.path(next))
            .map_err(|e| e.to_string())?;
        file.write_all(&writer.bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        fs::File::open(&self.root)
            .and_then(|directory| directory.sync_all())
            .map_err(|e| e.to_string())?;
        // Windows file contents are synced. Directory-entry survival across
        // machine power loss is not qualified by this portable journal API.
        self.sequence = next;
        self.digest = Sha256::digest(&writer.bytes).into();
        Ok(())
    }
}
