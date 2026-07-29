//! Typed policy receipts appended after durable WAL commits.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::policy::DecisionResult;

pub const RECEIPTS_FILE: &str = "wellfair/receipts.jsonl";
pub const MAX_LIST: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReceiptRecord {
    pub id: String,
    pub timestamp_unix: u32,
    pub qapp_id: String,
    pub record_id: String,
    pub decision: String,
    pub obligations: Vec<String>,
    pub checkpoint_hash: Option<String>,
}

pub struct ReceiptLog {
    path: PathBuf,
}

impl ReceiptLog {
    pub fn open(storage_root: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = storage_root.as_ref().join(RECEIPTS_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            OpenOptions::new().create(true).write(true).open(&path)?;
        }
        Ok(Self { path })
    }

    pub fn append(&self, record: &ReceiptRecord) -> std::io::Result<()> {
        let line =
            serde_json::to_string(record).map_err(|e| std::io::Error::other(e.to_string()))?;
        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        writeln!(file, "{line}")?;
        file.sync_all()?;
        Ok(())
    }

    pub fn list_recent(&self, limit: usize) -> std::io::Result<Vec<ReceiptRecord>> {
        let file = fs::File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<ReceiptRecord>(&line) {
                records.push(record);
            }
        }
        let keep = limit.min(MAX_LIST);
        if records.len() > keep {
            records.drain(0..records.len() - keep);
        }
        records.reverse();
        Ok(records)
    }
}

pub fn receipt_from_decision(
    qapp_id: &str,
    record_id: &str,
    timestamp_unix: u32,
    decision: &DecisionResult,
    checkpoint_hash: Option<[u8; 32]>,
) -> ReceiptRecord {
    let (decision_label, obligations) = match decision {
        DecisionResult::Permit { obligations } => ("permit".to_string(), obligations.clone()),
        DecisionResult::Deny { reasons } => ("deny".to_string(), reasons.clone()),
        DecisionResult::Prompt { .. } => ("prompt".to_string(), vec![]),
        DecisionResult::Suspend { required_approvals } => (
            "suspend".to_string(),
            vec![format!("approvals:{required_approvals}")],
        ),
    };
    ReceiptRecord {
        id: format!(
            "rcpt-{timestamp_unix}-{}",
            &record_id[record_id.len().saturating_sub(8)..]
        ),
        timestamp_unix,
        qapp_id: qapp_id.to_string(),
        record_id: record_id.to_string(),
        decision: decision_label,
        obligations,
        checkpoint_hash: checkpoint_hash.map(hex::encode),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_log_appends() {
        let dir = tempfile::tempdir().unwrap();
        let log = ReceiptLog::open(dir.path()).unwrap();
        let record = receipt_from_decision(
            "wellfair-health",
            "urn:wellfair:weight:x",
            100,
            &DecisionResult::Permit {
                obligations: vec!["emit_wal_receipt".into()],
            },
            None,
        );
        log.append(&record).unwrap();
        assert_eq!(log.list_recent(5).unwrap().len(), 1);
    }
}
