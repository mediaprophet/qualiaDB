//! Persisted owner consent grants evaluated by PolicyDecisionService.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::host_state::ConsentGrantDraft;

pub const CONSENTS_FILE: &str = "wellfair/consents.jsonl";
pub const MAX_LIST: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsentGrantRecord {
    pub id: String,
    pub recipient: String,
    pub purpose: String,
    pub fields: Vec<String>,
    pub scope: String,
    pub granted_at_unix: u32,
    pub expires_at_unix: Option<u64>,
    pub revoked: bool,
}

impl ConsentGrantRecord {
    pub fn from_draft(draft: &ConsentGrantDraft, scope: &str) -> Self {
        let granted_at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0);
        Self {
            id: format!("consent-{granted_at_unix}-{}", &draft.recipient),
            recipient: draft.recipient.clone(),
            purpose: draft.purpose.clone(),
            fields: draft.fields.clone(),
            scope: scope.to_string(),
            granted_at_unix,
            expires_at_unix: draft.expires_at_unix,
            revoked: false,
        }
    }

    pub fn is_active(&self, now_unix: u64) -> bool {
        if self.revoked {
            return false;
        }
        if let Some(exp) = self.expires_at_unix {
            if now_unix >= exp {
                return false;
            }
        }
        true
    }
}

pub struct ConsentStore {
    path: PathBuf,
}

impl ConsentStore {
    pub fn open(storage_root: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = storage_root.as_ref().join(CONSENTS_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            OpenOptions::new()
                .create(true)
                .write(true)
                .open(&path)?;
        }
        Ok(Self { path })
    }

    pub fn append(&self, grant: &ConsentGrantRecord) -> std::io::Result<()> {
        let line =
            serde_json::to_string(grant).map_err(|e| std::io::Error::other(e.to_string()))?;
        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        writeln!(file, "{line}")?;
        file.sync_all()?;
        Ok(())
    }

    pub fn list_all(&self) -> std::io::Result<Vec<ConsentGrantRecord>> {
        let file = fs::File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<ConsentGrantRecord>(&line) {
                records.push(record);
            }
        }
        Ok(records)
    }

    pub fn list_active(&self, now_unix: u64) -> std::io::Result<Vec<ConsentGrantRecord>> {
        let all = self.list_all()?;
        let mut active: Vec<_> = all
            .into_iter()
            .filter(|g| g.is_active(now_unix))
            .collect();
        let keep = MAX_LIST.min(active.len());
        if active.len() > keep {
            active.drain(0..active.len() - keep);
        }
        active.reverse();
        Ok(active)
    }

    pub fn revoke(&self, grant_id: &str) -> std::io::Result<bool> {
        let all = self.list_all()?;
        let mut found = false;
        let mut rewritten = Vec::with_capacity(all.len());
        for mut grant in all {
            if grant.id == grant_id && !grant.revoked {
                grant.revoked = true;
                found = true;
            }
            rewritten.push(grant);
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
            for grant in &rewritten {
                let line = serde_json::to_string(grant)
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

    #[test]
    fn consent_grant_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = ConsentStore::open(dir.path()).unwrap();
        let draft = ConsentGrantDraft {
            recipient: "wellfair-care".into(),
            purpose: "write_record".into(),
            fields: vec!["health.observation".into()],
            expires_at_unix: None,
        };
        let grant = ConsentGrantRecord::from_draft(&draft, "write_record");
        store.append(&grant).unwrap();
        let active = store.list_active(u64::MAX).unwrap();
        assert_eq!(active.len(), 1);
        assert!(store.revoke(&grant.id).unwrap());
        assert!(store.list_active(u64::MAX).unwrap().is_empty());
    }
}