//! Pending companion live-section requests and usage agreements (owner approval gate).

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use wellfare_core::live_share::{LiveSectionDecision, LiveSectionRequest, UsageAgreement};

use super::journal::{JournalEntry, WellfairJournal};
use super::sanctuary::is_sanctuary_protected_kind;

pub const LIVE_SHARE_REQUESTS_FILE: &str = "wellfair/live_share_requests.jsonl";
pub const USAGE_AGREEMENTS_FILE: &str = "wellfair/usage_agreements.jsonl";
pub const MAX_PENDING: usize = 64;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiveShareRequestStatus {
    Pending,
    Approved,
    Denied,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LiveShareRequestRecord {
    pub request: LiveSectionRequest,
    pub enqueued_at_unix: u64,
    pub status: LiveShareRequestStatus,
    /// Sanctuary-classified kinds present in the request (therapy_note, sanctuary_note, welfare_case).
    pub classified_kinds: Vec<String>,
    pub requires_owner_approval: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_kinds: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_at_unix: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deny_reason: Option<String>,
}

impl LiveShareRequestRecord {
    pub fn from_request(request: LiveSectionRequest, enqueued_at_unix: u64) -> Self {
        let classified_kinds: Vec<String> = request
            .requested_kinds
            .iter()
            .filter(|k| is_sanctuary_protected_kind(k))
            .cloned()
            .collect();
        let requires_owner_approval = !classified_kinds.is_empty();
        Self {
            request,
            enqueued_at_unix,
            status: LiveShareRequestStatus::Pending,
            classified_kinds,
            requires_owner_approval,
            approved: None,
            projection_kinds: None,
            decided_at_unix: None,
            deny_reason: None,
        }
    }
}

pub struct LiveShareStore {
    requests_path: PathBuf,
    agreements_path: PathBuf,
}

impl LiveShareStore {
    pub fn open(storage_root: impl AsRef<Path>) -> std::io::Result<Self> {
        let root = storage_root.as_ref();
        let requests_path = root.join(LIVE_SHARE_REQUESTS_FILE);
        let agreements_path = root.join(USAGE_AGREEMENTS_FILE);
        for path in [&requests_path, &agreements_path] {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            if !path.exists() {
                OpenOptions::new().create(true).write(true).open(path)?;
            }
        }
        Ok(Self {
            requests_path,
            agreements_path,
        })
    }

    pub fn enqueue_request(
        &self,
        request: LiveSectionRequest,
        now_unix: u64,
    ) -> std::io::Result<LiveShareRequestRecord> {
        let record = LiveShareRequestRecord::from_request(request, now_unix);
        let line =
            serde_json::to_string(&record).map_err(|e| std::io::Error::other(e.to_string()))?;
        let mut file = OpenOptions::new().append(true).open(&self.requests_path)?;
        writeln!(file, "{line}")?;
        file.sync_all()?;
        Ok(record)
    }

    pub fn list_pending(&self, limit: usize) -> std::io::Result<Vec<LiveSectionRequest>> {
        let all = self.load_requests()?;
        let mut pending: Vec<LiveSectionRequest> = all
            .into_iter()
            .filter(|r| r.status == LiveShareRequestStatus::Pending)
            .map(|r| r.request)
            .collect();
        let keep = limit.min(MAX_PENDING).min(pending.len());
        if pending.len() > keep {
            pending.drain(0..pending.len() - keep);
        }
        pending.reverse();
        Ok(pending)
    }

    pub fn get_request(&self, request_id: &str) -> std::io::Result<Option<LiveShareRequestRecord>> {
        Ok(self
            .load_requests()?
            .into_iter()
            .find(|r| r.request.id == request_id))
    }

    pub fn decide(
        &self,
        request_id: &str,
        approved: bool,
        projection_kinds: &[String],
        decided_at_unix: u64,
        deny_reason: Option<&str>,
    ) -> std::io::Result<LiveShareRequestRecord> {
        let all = self.load_requests()?;
        let mut found: Option<LiveShareRequestRecord> = None;
        let mut rewritten = Vec::with_capacity(all.len());
        for mut record in all {
            if record.request.id == request_id {
                if record.status != LiveShareRequestStatus::Pending {
                    return Err(std::io::Error::other(format!(
                        "live share request '{request_id}' already decided"
                    )));
                }
                record.status = if approved {
                    LiveShareRequestStatus::Approved
                } else {
                    LiveShareRequestStatus::Denied
                };
                record.approved = Some(approved);
                record.projection_kinds = if approved {
                    Some(projection_kinds.to_vec())
                } else {
                    Some(vec![])
                };
                record.decided_at_unix = Some(decided_at_unix);
                record.deny_reason = deny_reason.map(str::to_string);
                found = Some(record.clone());
            }
            rewritten.push(record);
        }
        let updated = found.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("live share request '{request_id}' not found"),
            )
        })?;
        self.rewrite_requests(&rewritten)?;
        Ok(updated)
    }

    pub fn save_usage_agreement(&self, agreement: &UsageAgreement) -> std::io::Result<()> {
        let line =
            serde_json::to_string(agreement).map_err(|e| std::io::Error::other(e.to_string()))?;
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.agreements_path)?;
        writeln!(file, "{line}")?;
        file.sync_all()?;
        Ok(())
    }

    pub fn get_usage_agreement(&self, device_id: &str) -> std::io::Result<Option<UsageAgreement>> {
        let file = fs::File::open(&self.agreements_path)?;
        let reader = BufReader::new(file);
        let mut latest: Option<UsageAgreement> = None;
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(agreement) = serde_json::from_str::<UsageAgreement>(&line) {
                if agreement.device_id == device_id {
                    latest = Some(agreement);
                }
            }
        }
        Ok(latest)
    }

    fn load_requests(&self) -> std::io::Result<Vec<LiveShareRequestRecord>> {
        let file = fs::File::open(&self.requests_path)?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<LiveShareRequestRecord>(&line) {
                records.push(record);
            }
        }
        Ok(records)
    }

    fn rewrite_requests(&self, records: &[LiveShareRequestRecord]) -> std::io::Result<()> {
        let tmp = self.requests_path.with_extension("jsonl.tmp");
        {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&tmp)?;
            for record in records {
                let line = serde_json::to_string(record)
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                writeln!(file, "{line}")?;
            }
            file.sync_all()?;
        }
        fs::rename(&tmp, &self.requests_path)?;
        Ok(())
    }
}

/// Fail-closed validation before owner approval: projection kinds must be requested,
/// and sanctuary-classified kinds require an unlocked (non-decoy) sanctuary session.
pub fn validate_live_share_decision(
    record: &LiveShareRequestRecord,
    approved: bool,
    projection_kinds: &[String],
    sanctuary_unlocked: bool,
) -> Result<(), String> {
    if !approved {
        return Ok(());
    }
    for kind in projection_kinds {
        if !record.request.requested_kinds.iter().any(|k| k == kind) {
            return Err(format!(
                "projection kind '{kind}' was not in the companion request (fail closed)"
            ));
        }
        if is_sanctuary_protected_kind(kind) && !sanctuary_unlocked {
            return Err(format!(
                "sanctuary protected kind '{kind}' requires explicit owner approval after sanctuary unlock"
            ));
        }
    }
    Ok(())
}

pub fn sanctuary_allows_classified_projection(prefs: &super::sanctuary::SanctuaryPrefs) -> bool {
    prefs.enabled && !prefs.locked && !prefs.decoy_session
}

pub fn live_share_request_journal_entry(
    record: &LiveShareRequestRecord,
    committed_unix: u32,
) -> JournalEntry {
    let sensitivity = if record.requires_owner_approval {
        "Classified"
    } else {
        "Restricted"
    };
    JournalEntry {
        id: format!("urn:wellfair:live_share_request:{}", record.request.id),
        kind: "live_share_request".into(),
        asserted_time_unix: record.enqueued_at_unix as u32,
        asserted_instant: wellfare_core::record::InstantBridge::from_coarse(
            record.enqueued_at_unix as u32,
        ),
        evidence_type: "SelfReported".into(),
        sensitivity: sensitivity.into(),
        blob_hash: None,
        source: "wellfair:live_share".into(),
        committed_unix,
        summary: Some(
            serde_json::json!({
                "request_id": record.request.id,
                "device_id": record.request.device_id,
                "purpose": record.request.purpose,
                "requested_kinds": record.request.requested_kinds,
                "classified_kinds": record.classified_kinds,
                "requires_owner_approval": record.requires_owner_approval,
            })
            .to_string(),
        ),
    }
}

pub fn live_share_decision_journal_entry(
    record: &LiveShareRequestRecord,
    committed_unix: u32,
) -> JournalEntry {
    let approved = record.approved.unwrap_or(false);
    let projection = record.projection_kinds.clone().unwrap_or_default();
    JournalEntry {
        id: format!("urn:wellfair:live_share_decision:{}", record.request.id),
        kind: "live_share_decision".into(),
        asserted_time_unix: record.decided_at_unix.unwrap_or(0) as u32,
        asserted_instant: wellfare_core::record::InstantBridge::from_coarse(
            record.decided_at_unix.unwrap_or(0) as u32,
        ),
        evidence_type: "SelfReported".into(),
        sensitivity: if projection.iter().any(|k| is_sanctuary_protected_kind(k)) {
            "Classified".into()
        } else {
            "Restricted".into()
        },
        blob_hash: None,
        source: "wellfair:live_share".into(),
        committed_unix,
        summary: Some(
            serde_json::json!({
                "request_id": record.request.id,
                "device_id": record.request.device_id,
                "approved": approved,
                "projection_kinds": projection,
                "classified_kinds": record.classified_kinds,
                "deny_reason": record.deny_reason,
            })
            .to_string(),
        ),
    }
}

/// Build the companion wire message after owner decision.
pub fn live_section_decision_from_record(record: &LiveShareRequestRecord) -> LiveSectionDecision {
    let decided_at = record.decided_at_unix.unwrap_or(0);
    if record.approved.unwrap_or(false) {
        LiveSectionDecision::approved(
            &record.request.id,
            record.projection_kinds.clone().unwrap_or_default(),
            decided_at,
        )
    } else {
        LiveSectionDecision::denied(
            &record.request.id,
            record
                .deny_reason
                .clone()
                .unwrap_or_else(|| "owner denied live share request".into()),
            decided_at,
        )
    }
}

pub fn append_live_share_journal(
    storage_root: impl AsRef<Path>,
    entry: &JournalEntry,
) -> Result<(), String> {
    WellfairJournal::open(storage_root.as_ref())
        .map_err(|e| e.to_string())?
        .append(entry)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wellfare_core::live_share::LiveSectionRequest;

    #[test]
    fn enqueue_flags_classified_kinds() {
        let dir = tempfile::tempdir().unwrap();
        let store = LiveShareStore::open(dir.path()).unwrap();
        let request = LiveSectionRequest::new(
            "req-1",
            "phone-1",
            "Desktop",
            "preview",
            vec!["conditions".into(), "therapy_note".into()],
            vec![],
            300,
        );
        let record = store.enqueue_request(request, 1_700_000_000).unwrap();
        assert!(record.requires_owner_approval);
        assert_eq!(record.classified_kinds, vec!["therapy_note"]);
        let pending = store.list_pending(8).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "req-1");
    }

    #[test]
    fn usage_agreement_latest_per_device() {
        let dir = tempfile::tempdir().unwrap();
        let store = LiveShareStore::open(dir.path()).unwrap();
        let first = UsageAgreement::new("phone-1", "vitals", vec!["sleep".into()], 100, 50);
        let second = UsageAgreement::new(
            "phone-1",
            "vitals v2",
            vec!["sleep".into(), "steps".into()],
            200,
            150,
        );
        store.save_usage_agreement(&first).unwrap();
        store.save_usage_agreement(&second).unwrap();
        let got = store.get_usage_agreement("phone-1").unwrap().unwrap();
        assert_eq!(got.purpose, "vitals v2");
    }
}
