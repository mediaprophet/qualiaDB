//! Append-only WellFair record journal (human-readable projection over WAL commits).

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use wellfare_core::conditions::journal_kind_for_record_id;
use wellfare_core::record::RecordEnvelope;

pub const JOURNAL_FILE: &str = "wellfair/journal.jsonl";
pub const MAX_LIST: usize = 256;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JournalEntry {
    pub id: String,
    pub kind: String,
    pub asserted_time_unix: u32,
    pub evidence_type: String,
    pub sensitivity: String,
    pub blob_hash: Option<String>,
    pub source: String,
    pub committed_unix: u32,
    /// Compact JSON projection for UI dashboards (sleep duration, weight kg, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

impl JournalEntry {
    pub fn from_envelope(
        envelope: &RecordEnvelope,
        source: &str,
        committed_unix: u32,
        summary: Option<String>,
    ) -> Self {
        let kind = infer_kind(&envelope.id);
        Self {
            id: envelope.id.clone(),
            kind,
            asserted_time_unix: envelope.asserted_time_unix,
            evidence_type: format!("{:?}", envelope.evidence_type),
            sensitivity: format!("{:?}", envelope.sensitivity),
            blob_hash: envelope.blob_hash.clone(),
            source: source.to_string(),
            committed_unix,
            summary,
        }
    }
}

fn infer_kind(record_id: &str) -> String {
    journal_kind_for_record_id(record_id).to_string()
}

pub struct WellfairJournal {
    path: PathBuf,
}

impl WellfairJournal {
    pub fn open(storage_root: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = storage_root.as_ref().join(JOURNAL_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            OpenOptions::new().create(true).write(true).open(&path)?;
        }
        Ok(Self { path })
    }

    pub fn append(&self, entry: &JournalEntry) -> std::io::Result<()> {
        let line =
            serde_json::to_string(entry).map_err(|e| std::io::Error::other(e.to_string()))?;
        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        writeln!(file, "{line}")?;
        file.sync_all()?;
        Ok(())
    }

    pub fn list_recent(&self, limit: usize) -> std::io::Result<Vec<JournalEntry>> {
        let file = fs::File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<JournalEntry>(&line) {
                entries.push(entry);
            }
        }
        let keep = limit.min(MAX_LIST);
        if entries.len() > keep {
            entries.drain(0..entries.len() - keep);
        }
        entries.reverse();
        Ok(entries)
    }

    pub fn count(&self) -> std::io::Result<usize> {
        let file = fs::File::open(&self.path)?;
        Ok(BufReader::new(file)
            .lines()
            .filter(|l| l.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false))
            .count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wellfare_core::record::{EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass};

    #[test]
    fn journal_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let journal = WellfairJournal::open(dir.path()).unwrap();
        let envelope = RecordEnvelope {
            id: "urn:wellfair:weight:abc".into(),
            owner_did: "did:wf:owner".into(),
            author_did: "did:wf:owner".into(),
            proxy_did: None,
            epistemic_status: EpistemicStatus::Asserted,
            evidence_type: EvidenceType::DeviceMeasured,
            sensitivity: SensitivityClass::Restricted,
            asserted_time_unix: 1_700_000_000,
            valid_time_start_unix: None,
            valid_time_end_unix: None,
            predecessor_id: None,
            blob_hash: Some("deadbeef".into()),
            tombstone: false,
        };
        let entry =
            JournalEntry::from_envelope(&envelope, "companion:phone-1", 1_700_000_100, None);
        journal.append(&entry).unwrap();
        let listed = journal.list_recent(10).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].kind, "weight");
    }
}
