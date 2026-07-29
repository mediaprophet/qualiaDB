//! Personal Core persisted fields (PRO-01..08) — emergency contacts and profile extensions.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const CONTACTS_FILE: &str = "wellfair/emergency_contacts.jsonl";
pub const MAX_CONTACTS: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmergencyContact {
    pub id: String,
    pub display_name: String,
    pub relationship: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub notes: Option<String>,
    pub created_at_unix: u32,
}

pub struct EmergencyContactStore {
    path: PathBuf,
}

impl EmergencyContactStore {
    pub fn open(storage_root: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = storage_root.as_ref().join(CONTACTS_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            OpenOptions::new().create(true).write(true).open(&path)?;
        }
        Ok(Self { path })
    }

    pub fn append(&self, contact: &EmergencyContact) -> std::io::Result<()> {
        let line =
            serde_json::to_string(contact).map_err(|e| std::io::Error::other(e.to_string()))?;
        let mut file = OpenOptions::new().append(true).open(&self.path)?;
        writeln!(file, "{line}")?;
        file.sync_all()?;
        Ok(())
    }

    pub fn list(&self) -> std::io::Result<Vec<EmergencyContact>> {
        let file = fs::File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut out = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(c) = serde_json::from_str::<EmergencyContact>(&line) {
                out.push(c);
            }
        }
        if out.len() > MAX_CONTACTS {
            out.drain(0..out.len() - MAX_CONTACTS);
        }
        Ok(out)
    }
}

pub fn new_contact_id(name: &str, unix: u32) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(format!("{name}:{unix}").as_bytes());
    format!("ec-{}", hex::encode(&digest[..4]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emergency_contact_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = EmergencyContactStore::open(dir.path()).unwrap();
        let c = EmergencyContact {
            id: new_contact_id("Alex", 100),
            display_name: "Alex".into(),
            relationship: "sibling".into(),
            phone: Some("+1-555-0100".into()),
            email: None,
            notes: None,
            created_at_unix: 100,
        };
        store.append(&c).unwrap();
        let listed = store.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].display_name, "Alex");
    }
}
