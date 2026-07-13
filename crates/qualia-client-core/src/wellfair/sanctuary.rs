//! Sanctuary vault state — setup, lock, decoy session (SAF-01..20; no destructive PIN).

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::journal::JournalEntry;

pub const SANCTUARY_PREFS_FILE: &str = "wellfair/sanctuary_prefs.json";

/// Journal kinds hidden while Sanctuary is locked (including decoy session).
pub const SANCTUARY_PROTECTED_KINDS: &[&str] = &[
    "therapy_note",
    "sanctuary_note",
    "welfare_case",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SanctuaryPrefs {
    pub enabled: bool,
    pub locked: bool,
    pub decoy_session: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub real_pin_hash_hex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decoy_pin_hash_hex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armed_at_unix: Option<u32>,
}

impl Default for SanctuaryPrefs {
    fn default() -> Self {
        Self {
            enabled: false,
            locked: false,
            decoy_session: false,
            real_pin_hash_hex: None,
            decoy_pin_hash_hex: None,
            armed_at_unix: None,
        }
    }
}

pub fn hash_pin(pin: &str) -> String {
    hex::encode(Sha256::digest(pin.as_bytes()))
}

pub fn load_prefs(storage_root: impl AsRef<Path>) -> SanctuaryPrefs {
    let path = storage_root.as_ref().join(SANCTUARY_PREFS_FILE);
    if !path.exists() {
        return SanctuaryPrefs::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_prefs(storage_root: impl AsRef<Path>, prefs: &SanctuaryPrefs) -> std::io::Result<()> {
    let path = storage_root.as_ref().join(SANCTUARY_PREFS_FILE);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(prefs)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    fs::write(&path, json)
}

pub fn setup_sanctuary(
    storage_root: impl AsRef<Path>,
    real_pin: &str,
    decoy_pin: &str,
    now_unix: u32,
) -> Result<SanctuaryPrefs, String> {
    if real_pin.len() < 4 || decoy_pin.len() < 4 {
        return Err("PIN must be at least 4 characters".into());
    }
    if real_pin == decoy_pin {
        return Err("Decoy PIN must differ from the real unlock PIN".into());
    }
    let prefs = SanctuaryPrefs {
        enabled: true,
        locked: false,
        decoy_session: false,
        real_pin_hash_hex: Some(hash_pin(real_pin)),
        decoy_pin_hash_hex: Some(hash_pin(decoy_pin)),
        armed_at_unix: Some(now_unix),
    };
    save_prefs(storage_root, &prefs).map_err(|e| e.to_string())?;
    Ok(prefs)
}

pub fn lock_sanctuary(storage_root: impl AsRef<Path>) -> Result<SanctuaryPrefs, String> {
    let mut prefs = load_prefs(&storage_root);
    if !prefs.enabled {
        return Err("Sanctuary is not set up".into());
    }
    prefs.locked = true;
    prefs.decoy_session = false;
    save_prefs(&storage_root, &prefs).map_err(|e| e.to_string())?;
    Ok(prefs)
}

pub fn unlock_sanctuary(storage_root: impl AsRef<Path>, pin: &str) -> Result<SanctuaryPrefs, String> {
    let mut prefs = load_prefs(&storage_root);
    if !prefs.enabled {
        return Err("Sanctuary is not set up".into());
    }
    let hash = hash_pin(pin);
    let real = prefs.real_pin_hash_hex.as_deref();
    let decoy = prefs.decoy_pin_hash_hex.as_deref();
    if Some(hash.as_str()) == real {
        prefs.locked = false;
        prefs.decoy_session = false;
        save_prefs(&storage_root, &prefs).map_err(|e| e.to_string())?;
        return Ok(prefs);
    }
    if Some(hash.as_str()) == decoy {
        prefs.locked = true;
        prefs.decoy_session = true;
        save_prefs(&storage_root, &prefs).map_err(|e| e.to_string())?;
        return Ok(prefs);
    }
    Err("Incorrect PIN".into())
}

pub fn is_sanctuary_protected_kind(kind: &str) -> bool {
    SANCTUARY_PROTECTED_KINDS.contains(&kind)
}

pub fn apply_sanctuary_projection(prefs: &SanctuaryPrefs, entries: Vec<JournalEntry>) -> Vec<JournalEntry> {
    if !prefs.enabled || !prefs.locked {
        return entries;
    }
    entries
        .into_iter()
        .filter(|e| !is_sanctuary_protected_kind(&e.kind))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wellfair::journal::JournalEntry;

    fn entry(kind: &str) -> JournalEntry {
        JournalEntry {
            id: format!("urn:test:{kind}"),
            kind: kind.into(),
            asserted_time_unix: 0,
            evidence_type: "SelfReported".into(),
            sensitivity: "Classified".into(),
            blob_hash: None,
            source: "test".into(),
            committed_unix: 0,
            summary: None,
        }
    }

    #[test]
    fn locked_sanctuary_hides_protected_kinds() {
        let prefs = SanctuaryPrefs {
            enabled: true,
            locked: true,
            decoy_session: false,
            ..Default::default()
        };
        let rows = vec![
            entry("weight"),
            entry("therapy_note"),
            entry("life_event"),
        ];
        let out = apply_sanctuary_projection(&prefs, rows);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|e| !is_sanctuary_protected_kind(&e.kind)));
    }

    #[test]
    fn decoy_pin_keeps_locked_with_decoy_flag() {
        let dir = tempfile::tempdir().unwrap();
        setup_sanctuary(dir.path(), "real-pin-1", "decoy-pin-2", 10).unwrap();
        lock_sanctuary(dir.path()).unwrap();
        let prefs = unlock_sanctuary(dir.path(), "decoy-pin-2").unwrap();
        assert!(prefs.locked);
        assert!(prefs.decoy_session);
    }

    #[test]
    fn real_pin_unlocks() {
        let dir = tempfile::tempdir().unwrap();
        setup_sanctuary(dir.path(), "real-pin-1", "decoy-pin-2", 10).unwrap();
        lock_sanctuary(dir.path()).unwrap();
        let prefs = unlock_sanctuary(dir.path(), "real-pin-1").unwrap();
        assert!(!prefs.locked);
        assert!(!prefs.decoy_session);
    }
}