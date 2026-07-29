//! Durable sync outbox: queued record commits awaiting upstream acknowledgement.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use wellfare_core::conditions::journal_kind_for_record_id;
use wellfare_core::record::RecordEnvelope;

pub const SYNC_OUTBOX_FILE: &str = "wellfair/sync_outbox.jsonl";
pub const MAX_LIST: usize = 256;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum SyncOutboxState {
    Queued,
    Sent,
    Acknowledged,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncOutboxEntry {
    pub operation_id: String,
    pub record_id: String,
    pub kind: String,
    pub committed_unix: u32,
    pub state: SyncOutboxState,
}

impl SyncOutboxEntry {
    pub fn from_envelope(envelope: &RecordEnvelope, committed_unix: u32) -> Self {
        Self {
            operation_id: uuid::Uuid::new_v4().to_string(),
            record_id: envelope.id.clone(),
            kind: journal_kind_for_record_id(&envelope.id).to_string(),
            committed_unix,
            state: SyncOutboxState::Queued,
        }
    }
}

pub struct SyncOutbox {
    path: PathBuf,
}

impl SyncOutbox {
    pub fn open(storage_root: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = storage_root.as_ref().join(SYNC_OUTBOX_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            OpenOptions::new().create(true).write(true).open(&path)?;
        }
        Ok(Self { path })
    }

    pub fn enqueue(&self, entry: &SyncOutboxEntry) -> std::io::Result<()> {
        let line =
            serde_json::to_string(entry).map_err(|e| std::io::Error::other(e.to_string()))?;
        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        writeln!(file, "{line}")?;
        file.sync_all()?;
        Ok(())
    }

    pub fn list_all(&self) -> std::io::Result<Vec<SyncOutboxEntry>> {
        let file = fs::File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<SyncOutboxEntry>(&line) {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    pub fn list_recent(&self, limit: usize) -> std::io::Result<Vec<SyncOutboxEntry>> {
        let mut entries = self.list_all()?;
        let keep = limit.min(MAX_LIST);
        if entries.len() > keep {
            entries.drain(0..entries.len() - keep);
        }
        entries.reverse();
        Ok(entries)
    }

    pub fn count_queued(&self) -> std::io::Result<usize> {
        Ok(self
            .list_all()?
            .into_iter()
            .filter(|e| e.state == SyncOutboxState::Queued)
            .count())
    }

    pub fn update_state(
        &self,
        operation_id: &str,
        state: SyncOutboxState,
    ) -> std::io::Result<bool> {
        let all = self.list_all()?;
        let mut found = false;
        let mut rewritten = Vec::with_capacity(all.len());
        for mut entry in all {
            if entry.operation_id == operation_id {
                entry.state = state;
                found = true;
            }
            rewritten.push(entry);
        }
        if !found {
            return Ok(false);
        }
        let tmp = self.path.with_extension("jsonl.tmp");
        {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&tmp)?;
            for entry in &rewritten {
                let line = serde_json::to_string(entry)
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                writeln!(file, "{line}")?;
            }
            file.sync_all()?;
        }
        fs::rename(&tmp, &self.path)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wellfare_core::record::{EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass};

    fn sample_envelope(id: &str) -> RecordEnvelope {
        RecordEnvelope {
            id: id.into(),
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
        }
    }

    #[test]
    fn sync_outbox_enqueue_and_list() {
        let dir = tempfile::tempdir().unwrap();
        let outbox = SyncOutbox::open(dir.path()).unwrap();
        let envelope = sample_envelope("urn:wellfair:weight:abc");
        let entry = SyncOutboxEntry::from_envelope(&envelope, 1_700_000_100);
        outbox.enqueue(&entry).unwrap();

        let listed = outbox.list_recent(10).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].record_id, "urn:wellfair:weight:abc");
        assert_eq!(listed[0].kind, "weight");
        assert_eq!(listed[0].committed_unix, 1_700_000_100);
        assert_eq!(listed[0].state, SyncOutboxState::Queued);
        assert!(!listed[0].operation_id.is_empty());
    }

    #[test]
    fn sync_outbox_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let outbox = SyncOutbox::open(dir.path()).unwrap();
        let envelope = sample_envelope("urn:wellfair:sleep:xyz");
        let entry = SyncOutboxEntry::from_envelope(&envelope, 42);
        outbox.enqueue(&entry).unwrap();

        let reopened = SyncOutbox::open(dir.path()).unwrap();
        let listed = reopened.list_recent(5).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].kind, "sleep");
        assert_eq!(reopened.count_queued().unwrap(), 1);
    }

    #[test]
    fn sync_outbox_state_transition() {
        let dir = tempfile::tempdir().unwrap();
        let outbox = SyncOutbox::open(dir.path()).unwrap();
        let envelope = sample_envelope("urn:wellfair:steps:1");
        let entry = SyncOutboxEntry::from_envelope(&envelope, 99);
        outbox.enqueue(&entry).unwrap();

        assert!(outbox
            .update_state(&entry.operation_id, SyncOutboxState::Sent)
            .unwrap());
        let listed = outbox.list_recent(1).unwrap();
        assert_eq!(listed[0].state, SyncOutboxState::Sent);
        assert_eq!(outbox.count_queued().unwrap(), 0);

        assert!(outbox
            .update_state(&entry.operation_id, SyncOutboxState::Acknowledged)
            .unwrap());
        assert_eq!(
            outbox.list_recent(1).unwrap()[0].state,
            SyncOutboxState::Acknowledged
        );
    }

    #[test]
    fn sync_outbox_serializes_pascal_case_state() {
        let dir = tempfile::tempdir().unwrap();
        let outbox = SyncOutbox::open(dir.path()).unwrap();
        let entry =
            SyncOutboxEntry::from_envelope(&sample_envelope("urn:wellfair:heart_rate:hr1"), 1);
        outbox.enqueue(&entry).unwrap();

        let raw = fs::read_to_string(dir.path().join(SYNC_OUTBOX_FILE)).unwrap();
        assert!(raw.contains("\"state\":\"Queued\""));
        assert!(raw.contains("\"kind\":\"heart_rate\""));
    }
}
