//! Local medication reminder prefs and due-slot computation (Q6 MED-01..13).

use std::fs;
use std::path::Path;

use chrono::{NaiveTime, Timelike};
use serde::{Deserialize, Serialize};

use super::journal::JournalEntry;

pub const PREFS_FILE: &str = "wellfair/med_reminder_prefs.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MedReminderPrefs {
    pub enabled: bool,
    pub permission_granted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_granted_at_unix: Option<u32>,
}

impl Default for MedReminderPrefs {
    fn default() -> Self {
        Self {
            enabled: false,
            permission_granted: false,
            permission_granted_at_unix: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DueMedReminder {
    pub medication_id: String,
    pub medication_name: String,
    pub schedule_slot: String,
    /// Minutes until due (negative = overdue today).
    pub minutes_until_due: i32,
}

pub fn load_prefs(storage_root: impl AsRef<Path>) -> MedReminderPrefs {
    let path = storage_root.as_ref().join(PREFS_FILE);
    if !path.exists() {
        return MedReminderPrefs::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_prefs(storage_root: impl AsRef<Path>, prefs: &MedReminderPrefs) -> std::io::Result<()> {
    let path = storage_root.as_ref().join(PREFS_FILE);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(prefs)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    fs::write(&path, json)
}

fn parse_hhmm(slot: &str) -> Option<NaiveTime> {
    let parts: Vec<_> = slot.trim().split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let h: u32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    NaiveTime::from_hms_opt(h, m, 0)
}

/// Compute due/overdue slots for active medications from journal summaries.
pub fn compute_due_reminders(
    journal: &[JournalEntry],
    now_local: NaiveTime,
    window_minutes: i32,
) -> Vec<DueMedReminder> {
    let now_mins = (now_local.hour() * 60 + now_local.minute()) as i32;
    let mut out = Vec::new();

    for entry in journal {
        if entry.kind != "medication" {
            continue;
        }
        let Some(summary) = &entry.summary else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(summary) else {
            continue;
        };
        if json.get("ceased").and_then(|v| v.as_bool()).unwrap_or(false) {
            continue;
        }
        let name = json
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("medication")
            .to_string();
        let slots = json
            .get("schedule_times")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        for slot in slots {
            let Some(t) = parse_hhmm(&slot) else {
                continue;
            };
            let slot_mins = (t.hour() * 60 + t.minute()) as i32;
            let minutes_until = slot_mins - now_mins;
            if minutes_until.abs() <= window_minutes {
                out.push(DueMedReminder {
                    medication_id: entry.id.clone(),
                    medication_name: name.clone(),
                    schedule_slot: slot,
                    minutes_until_due: minutes_until,
                });
            }
        }
    }

    out.sort_by_key(|r| r.minutes_until_due);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn med_entry(id: &str, summary: &str) -> JournalEntry {
        JournalEntry {
            id: id.into(),
            kind: "medication".into(),
            asserted_time_unix: 0,
            evidence_type: "SelfReported".into(),
            sensitivity: "Restricted".into(),
            blob_hash: None,
            source: "test".into(),
            committed_unix: 0,
            summary: Some(summary.into()),
        }
    }

    #[test]
    fn prefs_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut prefs = MedReminderPrefs::default();
        prefs.permission_granted = true;
        prefs.enabled = true;
        prefs.permission_granted_at_unix = Some(100);
        save_prefs(dir.path(), &prefs).unwrap();
        let loaded = load_prefs(dir.path());
        assert_eq!(loaded, prefs);
    }

    #[test]
    fn due_reminder_within_window() {
        let summary = r#"{"name":"Metformin","schedule_times":["08:00","20:00"],"ceased":false}"#;
        let journal = vec![med_entry("urn:wellfair:medication:x", summary)];
        let now = NaiveTime::from_hms_opt(8, 5, 0).unwrap();
        let due = compute_due_reminders(&journal, now, 30);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].schedule_slot, "08:00");
        assert_eq!(due[0].minutes_until_due, -5);
    }

    #[test]
    fn ceased_medication_excluded() {
        let summary = r#"{"name":"Old","schedule_times":["08:00"],"ceased":true}"#;
        let journal = vec![med_entry("urn:wellfair:medication:y", summary)];
        let now = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        assert!(compute_due_reminders(&journal, now, 30).is_empty());
    }
}